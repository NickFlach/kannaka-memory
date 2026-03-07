#!/usr/bin/env node
const mysql = require('mysql2/promise');

async function checkSchema() {
    try {
        const connection = await mysql.createConnection({
            host: '127.0.0.1',
            port: 3307,
            database: 'dolt-memory',
            user: 'root',
            password: ''
        });

        console.log('Checking memories table schema:');
        const [memorySchema] = await connection.execute('DESCRIBE memories');
        console.table(memorySchema);
        
        console.log('\nChecking skip_links table schema:');
        const [skipLinksSchema] = await connection.execute('DESCRIBE skip_links');
        console.table(skipLinksSchema);
        
        await connection.end();
    } catch (error) {
        console.error('Error:', error.message);
    }
}

checkSchema();