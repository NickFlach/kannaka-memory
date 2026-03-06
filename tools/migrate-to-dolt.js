#!/usr/bin/env node
/**
 * Kannaka Memory Migration to Dolt
 * 
 * Migrates existing kannaka-memory bincode snapshots to Dolt database
 * Usage: node migrate-to-dolt.js
 */

const { execSync, spawn } = require('child_process');
const mysql = require('mysql2/promise');
const path = require('path');
const fs = require('fs');

const KANNAKA_CLI = 'C:\\Users\\nickf\\Source\\kannaka-memory\\target\\release\\kannaka.exe';
const DOLT_DB_PATH = 'C:\\Users\\nickf\\.kannaka\\dolt-memory';
const DOLT_HOST = '127.0.0.1';
const DOLT_PORT = 3307;
const DOLT_DB = 'dolt-memory';

async function main() {
    console.log('🧠 Kannaka Memory Migration to Dolt');
    console.log('=====================================\n');

    // 1. Check kannaka CLI exists and get status
    console.log('📊 Checking kannaka-memory status...');
    try {
        const statusOutput = execSync(`"${KANNAKA_CLI}" status`, { 
            encoding: 'utf-8',
            cwd: process.cwd()
        });
        console.log(statusOutput);
    } catch (error) {
        console.error('❌ Failed to get kannaka status:', error.message);
        process.exit(1);
    }

    // 2. Start Dolt SQL server if not running
    console.log('🚀 Starting Dolt SQL server...');
    const doltServer = startDoltServer();
    
    // Wait a moment for server to start
    await new Promise(resolve => setTimeout(resolve, 2000));

    try {
        // 3. Connect to Dolt
        console.log('🔌 Connecting to Dolt database...');
        const connection = await mysql.createConnection({
            host: DOLT_HOST,
            port: DOLT_PORT,
            database: DOLT_DB,
            user: 'root',
            password: ''
        });

        // 4. Get all memories from kannaka CLI
        console.log('🔍 Retrieving memories from kannaka...');
        let memories = [];
        
        try {
            const recallOutput = execSync(`"${KANNAKA_CLI}" recall "*" --limit 1000 --format json`, {
                encoding: 'utf-8',
                maxBuffer: 10 * 1024 * 1024, // 10MB buffer
                cwd: process.cwd()
            });
            
            // Parse JSON output (assuming each line is a JSON memory)
            const lines = recallOutput.trim().split('\n').filter(line => line.trim());
            for (const line of lines) {
                try {
                    const memory = JSON.parse(line);
                    memories.push(memory);
                } catch (parseError) {
                    console.warn('⚠️ Failed to parse memory line:', line.substring(0, 100));
                }
            }
        } catch (error) {
            console.error('❌ Failed to recall memories:', error.message);
            // Continue with empty memories array
        }

        console.log(`📝 Found ${memories.length} memories to migrate`);

        // 5. Clear existing data and insert memories
        if (memories.length > 0) {
            console.log('🗑️ Clearing existing data...');
            await connection.execute('DELETE FROM memories');
            await connection.execute('DELETE FROM skip_links');

            console.log('📥 Inserting memories...');
            let successCount = 0;
            
            for (const memory of memories) {
                try {
                    // Prepare memory data for insertion
                    const memoryData = {
                        id: memory.id || generateId(),
                        content: memory.content || '',
                        amplitude: memory.amplitude || 1.0,
                        frequency: memory.frequency || 1.0,
                        phase: memory.phase || 0.0,
                        decay_rate: memory.decay_rate || 0.01,
                        created_at: memory.created_at || new Date().toISOString(),
                        layer_depth: memory.layer_depth || 0,
                        hallucinated: memory.hallucinated || false,
                        parents: memory.parents ? JSON.stringify(memory.parents) : null,
                        vector_data: memory.vector ? encodeVector(memory.vector) : '',
                        xi_signature: memory.xi_signature ? encodeVector(memory.xi_signature) : null,
                        geometry: memory.geometry ? JSON.stringify(memory.geometry) : null
                    };

                    await connection.execute(
                        `INSERT INTO memories (
                            id, content, amplitude, frequency, phase, decay_rate, 
                            created_at, layer_depth, hallucinated, parents, 
                            vector_data, xi_signature, geometry
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
                        [
                            memoryData.id, memoryData.content, memoryData.amplitude,
                            memoryData.frequency, memoryData.phase, memoryData.decay_rate,
                            memoryData.created_at, memoryData.layer_depth, memoryData.hallucinated,
                            memoryData.parents, memoryData.vector_data, memoryData.xi_signature,
                            memoryData.geometry
                        ]
                    );

                    successCount++;
                    if (successCount % 10 === 0) {
                        console.log(`   ✅ Inserted ${successCount}/${memories.length} memories`);
                    }
                } catch (insertError) {
                    console.error(`❌ Failed to insert memory ${memory.id}:`, insertError.message);
                }
            }

            console.log(`✅ Successfully inserted ${successCount}/${memories.length} memories`);
        }

        // 6. Update metadata
        await connection.execute(
            'INSERT INTO metadata (key_name, value_text) VALUES (?, ?) ON DUPLICATE KEY UPDATE value_text = VALUES(value_text)',
            ['migration_date', new Date().toISOString()]
        );

        await connection.execute(
            'INSERT INTO metadata (key_name, value_text) VALUES (?, ?) ON DUPLICATE KEY UPDATE value_text = VALUES(value_text)',
            ['migrated_count', memories.length.toString()]
        );

        await connection.end();
        console.log('🔌 Database connection closed');

        // 7. Commit to Dolt
        console.log('💾 Committing migration to Dolt...');
        execSync('dolt add .', { cwd: DOLT_DB_PATH });
        execSync('dolt commit -m "initial import from bincode store"', { cwd: DOLT_DB_PATH });

        console.log('\n🎉 Migration completed successfully!');
        console.log(`📊 Migrated ${memories.length} memories to Dolt database`);

    } catch (error) {
        console.error('❌ Migration failed:', error.message);
        process.exit(1);
    } finally {
        // Stop Dolt server
        if (doltServer) {
            console.log('🛑 Stopping Dolt SQL server...');
            doltServer.kill('SIGTERM');
        }
    }
}

function startDoltServer() {
    console.log('   Starting server on port 3307...');
    const server = spawn('dolt', ['sql-server', '-H', '0.0.0.0', '-P', '3307'], {
        cwd: DOLT_DB_PATH,
        stdio: 'inherit'
    });
    
    server.on('error', (error) => {
        console.error('❌ Failed to start Dolt server:', error.message);
    });
    
    return server;
}

function generateId() {
    // Generate a simple UUID-like string
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
        const r = Math.random() * 16 | 0;
        const v = c == 'x' ? r : (r & 0x3 | 0x8);
        return v.toString(16);
    });
}

function encodeVector(vector) {
    // For now, JSON encode the vector array
    // In production, we might want to use a more efficient encoding
    return JSON.stringify(vector);
}

// Handle process termination gracefully
process.on('SIGINT', () => {
    console.log('\n🛑 Migration interrupted');
    process.exit(1);
});

process.on('SIGTERM', () => {
    console.log('\n🛑 Migration terminated');
    process.exit(1);
});

if (require.main === module) {
    main().catch(console.error);
}