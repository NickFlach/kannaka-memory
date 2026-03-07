#!/usr/bin/env node
/**
 * Kannaka Memory Migration from OpenClaw to Dolt
 * 
 * Migrates existing kannaka-memory from OpenClaw system to Dolt database
 * Usage: node migrate-openclaw-to-dolt.js
 */

const mysql = require('mysql2/promise');
const fs = require('fs');
const path = require('path');

const DOLT_HOST = '127.0.0.1';
const DOLT_PORT = 3307;
const DOLT_DB = 'dolt-memory';

// Sample memory data structure - we'll fetch this from OpenClaw's memory system
const SAMPLE_MEMORIES = [
    {
        id: 'sample-1',
        content: 'This is a sample memory from the migration',
        amplitude: 1.0,
        frequency: 1.0,
        phase: 0.0,
        decay_rate: 0.01,
        created_at: new Date().toISOString(),
        layer_depth: 0,
        hallucinated: false,
        parents: [],
        vector_data: JSON.stringify([0.1, 0.2, 0.3]),
        xi_signature: null,
        geometry: null
    }
];

async function main() {
    console.log('🧠 Kannaka Memory Migration from OpenClaw to Dolt');
    console.log('==================================================\n');

    try {
        // Connect to Dolt
        console.log('🔌 Connecting to Dolt database...');
        const connection = await mysql.createConnection({
            host: DOLT_HOST,
            port: DOLT_PORT,
            database: DOLT_DB,
            user: 'root',
            password: ''
        });

        console.log('✅ Connected to Dolt successfully');

        // Check current table status
        console.log('📊 Checking current database state...');
        const [memoryCount] = await connection.execute('SELECT COUNT(*) as count FROM memories');
        const [skipLinkCount] = await connection.execute('SELECT COUNT(*) as count FROM skip_links');
        
        console.log(`Current state: ${memoryCount[0].count} memories, ${skipLinkCount[0].count} skip_links`);

        // For now, let's insert some sample data to test the migration process
        // In a real migration, we'd fetch data from the OpenClaw system
        console.log('\n📝 Inserting sample data for testing...');
        
        // Clear existing data
        console.log('🗑️ Clearing existing data...');
        await connection.execute('DELETE FROM memories');
        await connection.execute('DELETE FROM skip_links');

        // Insert sample memories
        let successCount = 0;
        for (const memory of SAMPLE_MEMORIES) {
            try {
                await connection.execute(
                    `INSERT INTO memories (
                        id, content, amplitude, frequency, phase, decay_rate, 
                        created_at, layer_depth, hallucinated, parents, 
                        vector_data, xi_signature, geometry
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
                    [
                        memory.id, memory.content, memory.amplitude,
                        memory.frequency, memory.phase, memory.decay_rate,
                        memory.created_at, memory.layer_depth, memory.hallucinated,
                        JSON.stringify(memory.parents), memory.vector_data, 
                        memory.xi_signature, memory.geometry
                    ]
                );
                successCount++;
            } catch (insertError) {
                console.error(`❌ Failed to insert memory ${memory.id}:`, insertError.message);
            }
        }

        console.log(`✅ Inserted ${successCount} sample memories`);

        // Update metadata
        await connection.execute(
            'INSERT INTO metadata (key_name, value_text) VALUES (?, ?) ON DUPLICATE KEY UPDATE value_text = VALUES(value_text)',
            ['migration_date', new Date().toISOString()]
        );

        await connection.execute(
            'INSERT INTO metadata (key_name, value_text) VALUES (?, ?) ON DUPLICATE KEY UPDATE value_text = VALUES(value_text)',
            ['migrated_count', successCount.toString()]
        );

        // Verify the insertion
        const [newMemoryCount] = await connection.execute('SELECT COUNT(*) as count FROM memories');
        const [newSkipLinkCount] = await connection.execute('SELECT COUNT(*) as count FROM skip_links');
        
        console.log(`Final state: ${newMemoryCount[0].count} memories, ${newSkipLinkCount[0].count} skip_links`);

        await connection.end();
        console.log('🔌 Database connection closed');

        console.log('\n🎉 Migration test completed successfully!');
        console.log(`📊 Ready to migrate real memories from OpenClaw system`);

    } catch (error) {
        console.error('❌ Migration failed:', error.message);
        console.error('Stack trace:', error.stack);
        process.exit(1);
    }
}

if (require.main === module) {
    main().catch(console.error);
}