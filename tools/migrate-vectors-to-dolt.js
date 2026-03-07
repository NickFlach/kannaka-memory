#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const mysql = require('mysql2/promise');
const { spawn } = require('child_process');

const DOLT_DIR = 'C:\\Users\\nickf\\.kannaka\\dolt-memory';
const EXPORT_FILE = path.join(__dirname, 'full-export.json');
const PORT = 3307;

let doltProcess = null;

async function startDoltServer() {
    console.log('Starting Dolt SQL server...');
    
    return new Promise((resolve, reject) => {
        doltProcess = spawn('dolt', ['sql-server', '--port', PORT.toString()], {
            cwd: DOLT_DIR,
            stdio: ['ignore', 'pipe', 'pipe']
        });

        doltProcess.stdout.on('data', (data) => {
            console.log(`[dolt] ${data.toString().trim()}`);
            if (data.toString().includes('Server ready')) {
                resolve();
            }
        });

        doltProcess.stderr.on('data', (data) => {
            const msg = data.toString().trim();
            console.error(`[dolt-err] ${msg}`);
            if (msg.includes('bind: Only one usage of each socket address')) {
                resolve(); // Server already running
            }
        });

        doltProcess.on('error', reject);
        
        // Give it some time to start up
        setTimeout(resolve, 3000);
    });
}

async function stopDoltServer() {
    if (doltProcess) {
        console.log('Stopping Dolt server...');
        doltProcess.kill('SIGTERM');
        doltProcess = null;
    }
}

async function connectToDatabase() {
    console.log('Connecting to database...');
    const connection = await mysql.createConnection({
        host: 'localhost',
        port: PORT,
        user: 'root',
        password: '',
        database: 'kannaka_memory'
    });
    return connection;
}

async function migrateMemories() {
    try {
        console.log(`Reading export file: ${EXPORT_FILE}`);
        if (!fs.existsSync(EXPORT_FILE)) {
            throw new Error(`Export file not found: ${EXPORT_FILE}`);
        }

        const exportData = JSON.parse(fs.readFileSync(EXPORT_FILE, 'utf8'));
        console.log(`Loaded ${exportData.length} memories from export`);

        await startDoltServer();
        const db = await connectToDatabase();

        console.log('Clearing existing skip_links...');
        await db.execute('DELETE FROM skip_links');

        console.log('Migrating memories with full vectors...');
        let updated = 0;
        let inserted = 0;
        let skipLinksCreated = 0;

        for (const memory of exportData) {
            try {
                // Convert vector array to JSON string
                const vectorData = JSON.stringify(memory.vector);
                const xiSignature = JSON.stringify(memory.xi_signature);
                const geometry = JSON.stringify(memory.geometry);
                const parents = JSON.stringify(memory.parents);

                // Try to update existing record first
                const [updateResult] = await db.execute(`
                    UPDATE memories 
                    SET vector_data = ?, 
                        xi_signature = ?, 
                        geometry = ?, 
                        amplitude = ?, 
                        frequency = ?, 
                        phase = ?, 
                        decay_rate = ?, 
                        created_at = ?, 
                        layer_depth = ?, 
                        hallucinated = ?, 
                        parents = ?
                    WHERE id = ?
                `, [
                    vectorData,
                    xiSignature, 
                    geometry,
                    memory.amplitude,
                    memory.frequency,
                    memory.phase,
                    memory.decay_rate,
                    memory.created_at,
                    memory.layer_depth,
                    memory.hallucinated,
                    parents,
                    memory.id
                ]);

                if (updateResult.affectedRows > 0) {
                    updated++;
                } else {
                    // Insert new record if update didn't affect any rows
                    await db.execute(`
                        INSERT INTO memories (
                            id, content, vector_data, xi_signature, geometry,
                            amplitude, frequency, phase, decay_rate, created_at,
                            layer_depth, hallucinated, parents
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    `, [
                        memory.id,
                        memory.content,
                        vectorData,
                        xiSignature,
                        geometry,
                        memory.amplitude,
                        memory.frequency,
                        memory.phase,
                        memory.decay_rate,
                        memory.created_at,
                        memory.layer_depth,
                        memory.hallucinated,
                        parents
                    ]);
                    inserted++;
                }

                // Insert connections into skip_links table
                for (const connection of memory.connections) {
                    await db.execute(`
                        INSERT INTO skip_links (source_id, target_id, strength, span)
                        VALUES (?, ?, ?, ?)
                    `, [
                        memory.id,
                        connection.target_id,
                        connection.strength,
                        connection.span
                    ]);
                    skipLinksCreated++;
                }

                if ((updated + inserted) % 10 === 0) {
                    console.log(`Processed ${updated + inserted} memories...`);
                }

            } catch (err) {
                console.error(`Error processing memory ${memory.id}:`, err.message);
            }
        }

        console.log(`Migration complete: ${updated} updated, ${inserted} inserted, ${skipLinksCreated} skip links created`);

        // Commit to Dolt
        console.log('Committing to Dolt...');
        await db.execute("CALL DOLT_ADD('.')");
        await db.execute("CALL DOLT_COMMIT('-m', 'full vector migration: 151 memories with 10000-dim hypervectors')");

        // Verify counts
        console.log('Verifying migration...');
        const [memoryCount] = await db.execute('SELECT COUNT(*) as count FROM memories');
        const [skipLinkCount] = await db.execute('SELECT COUNT(*) as count FROM skip_links');
        const [vectorSample] = await db.execute('SELECT id, LENGTH(vector_data) as vec_len FROM memories LIMIT 3');

        console.log(`Final counts:`);
        console.log(`  Memories: ${memoryCount[0].count}`);
        console.log(`  Skip links: ${skipLinkCount[0].count}`);
        console.log(`Sample vector lengths:`, vectorSample.map(r => `${r.id}: ${r.vec_len} bytes`).join(', '));

        await db.end();

    } catch (error) {
        console.error('Migration failed:', error);
        throw error;
    } finally {
        await stopDoltServer();
    }
}

// Handle graceful shutdown
process.on('SIGINT', async () => {
    console.log('\nShutting down...');
    await stopDoltServer();
    process.exit(0);
});

process.on('SIGTERM', async () => {
    await stopDoltServer();
    process.exit(0);
});

// Run the migration
if (require.main === module) {
    migrateMemories()
        .then(() => {
            console.log('✅ Full vector migration completed successfully!');
            process.exit(0);
        })
        .catch(error => {
            console.error('❌ Migration failed:', error);
            process.exit(1);
        });
}

module.exports = { migrateMemories };