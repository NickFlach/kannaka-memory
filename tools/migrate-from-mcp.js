const mysql = require('mysql2/promise');
const fs = require('fs').promises;
const path = require('path');

async function migrate() {
    try {
        // Connect to Dolt database
        const connection = await mysql.createConnection({
            host: 'localhost',
            port: 3307,
            user: 'root',
            database: 'dolt-memory'
        });
        
        console.log('Connected to Dolt database');
        
        // Load memories from JSON export
        const memoriesPath = path.join(__dirname, 'memories-export.json');
        const memoriesData = JSON.parse(await fs.readFile(memoriesPath, 'utf8'));
        
        console.log(`Loaded ${memoriesData.length} memories from export`);
        
        // Base timestamp for calculations (2026-03-06 17:41 UTC)
        const baseTime = new Date('2026-03-06T17:41:00.000Z');
        
        let insertedCount = 0;
        
        for (const memory of memoriesData) {
            // Calculate created_at timestamp from age_hours
            const ageMs = memory.age_hours * 60 * 60 * 1000; // Convert hours to milliseconds
            const createdAt = new Date(baseTime.getTime() - ageMs);
            
            // Default values as specified
            const amplitude = 1.0;
            const frequency = 0.5;
            const phase = 0.0;
            const decay_rate = 0.001;
            const vector_data = JSON.stringify([]);  // Empty JSON array as placeholder
            const xi_signature = JSON.stringify([]);
            const geometry = null;
            const parents = JSON.stringify([]);
            
            // Insert memory into database
            const insertQuery = `
                INSERT INTO memories (
                    id, content, amplitude, frequency, phase, decay_rate,
                    vector_data, xi_signature, geometry, created_at, parents, 
                    layer_depth, hallucinated
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            `;
            
            const values = [
                memory.id,
                memory.content,
                amplitude,
                frequency,
                phase,
                decay_rate,
                vector_data,
                xi_signature,
                geometry,
                createdAt,
                parents,
                memory.layer_depth,
                memory.hallucinated ? 1 : 0  // Convert boolean to integer
            ];
            
            try {
                await connection.execute(insertQuery, values);
                insertedCount++;
                
                if (insertedCount % 10 === 0) {
                    console.log(`Inserted ${insertedCount}/${memoriesData.length} memories...`);
                }
            } catch (error) {
                console.error(`Failed to insert memory ${memory.id}:`, error.message);
            }
        }
        
        console.log(`Successfully inserted ${insertedCount} memories`);
        
        // Commit changes to Dolt
        console.log('Committing changes to Dolt...');
        await connection.execute("CALL DOLT_ADD('.')");
        await connection.execute("CALL DOLT_COMMIT('-m', 'full migration: 151 memories from MCP export')");
        console.log('Changes committed to Dolt');
        
        // Verify the migration
        const [countResult] = await connection.execute('SELECT COUNT(*) as count FROM memories');
        console.log(`Verification: ${countResult[0].count} memories in database`);
        
        // Show some sample data
        const [sampleResults] = await connection.execute(`
            SELECT id, LEFT(content, 60) as content_preview, layer_depth, 
                   created_at, hallucinated 
            FROM memories 
            ORDER BY created_at DESC 
            LIMIT 5
        `);
        
        console.log('\nSample of migrated data:');
        sampleResults.forEach((row, index) => {
            console.log(`${index + 1}. ${row.id}`);
            console.log(`   Content: ${row.content_preview}...`);
            console.log(`   Layer: ${row.layer_depth}, Created: ${row.created_at}, Hallucinated: ${row.hallucinated}`);
            console.log('');
        });
        
        await connection.end();
        console.log('Migration completed successfully!');
        
    } catch (error) {
        console.error('Migration failed:', error);
        process.exit(1);
    }
}

// Run migration
migrate();