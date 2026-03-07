#!/usr/bin/env node
const mysql = require('mysql2/promise');

async function setupKannakaDb() {
    try {
        // Connect without specifying a database
        const connection = await mysql.createConnection({
            host: '127.0.0.1',
            port: 3307,
            user: 'root',
            password: ''
        });

        console.log('🔌 Connected to Dolt server');
        
        // List existing databases
        const [databases] = await connection.execute('SHOW DATABASES');
        console.log('📋 Existing databases:');
        console.table(databases);
        
        // Create kannaka_memory database
        console.log('🏗️ Creating kannaka_memory database...');
        await connection.execute('CREATE DATABASE IF NOT EXISTS kannaka_memory');
        
        // Switch to the new database
        await connection.execute('USE kannaka_memory');
        
        // Create tables using our working schema
        console.log('📋 Creating tables...');
        await connection.execute(`
            CREATE TABLE memories (
                id VARCHAR(36) PRIMARY KEY,
                content TEXT NOT NULL,
                amplitude FLOAT NOT NULL,
                frequency FLOAT NOT NULL,
                phase FLOAT NOT NULL,
                decay_rate FLOAT NOT NULL,
                created_at DATETIME NOT NULL,
                layer_depth TINYINT UNSIGNED NOT NULL,
                hallucinated BOOLEAN DEFAULT FALSE,
                parents JSON,
                vector_data TEXT NOT NULL,
                xi_signature TEXT,
                geometry JSON
            )
        `);
        
        await connection.execute(`
            CREATE TABLE skip_links (
                source_id VARCHAR(36) NOT NULL,
                target_id VARCHAR(36) NOT NULL,
                weight FLOAT NOT NULL,
                link_type VARCHAR(32) NOT NULL,
                created_at DATETIME NOT NULL,
                PRIMARY KEY (source_id, target_id),
                INDEX idx_target (target_id)
            )
        `);
        
        await connection.execute(`
            CREATE TABLE metadata (
                key_name VARCHAR(64) PRIMARY KEY,
                value_text TEXT
            )
        `);
        
        // Copy data from dolt-memory database
        console.log('📊 Copying data from dolt-memory...');
        await connection.execute(`
            INSERT INTO memories 
            SELECT * FROM \`dolt-memory\`.memories
        `);
        
        await connection.execute(`
            INSERT INTO skip_links 
            SELECT * FROM \`dolt-memory\`.skip_links
        `);
        
        await connection.execute(`
            INSERT INTO metadata 
            SELECT * FROM \`dolt-memory\`.metadata
        `);
        
        // Verify the copy
        const [memoryCount] = await connection.execute('SELECT COUNT(*) as count FROM memories');
        const [skipLinkCount] = await connection.execute('SELECT COUNT(*) as count FROM skip_links');
        
        console.log(`✅ Copied ${memoryCount[0].count} memories and ${skipLinkCount[0].count} skip_links`);
        
        await connection.end();
        console.log('✅ kannaka_memory database setup complete!');
        
    } catch (error) {
        console.error('❌ Setup failed:', error.message);
    }
}

setupKannakaDb();