#!/usr/bin/env node
const mysql = require('mysql2/promise');

async function verifyMigration() {
    try {
        const connection = await mysql.createConnection({
            host: '127.0.0.1',
            port: 3307,
            database: 'dolt-memory',
            user: 'root',
            password: ''
        });

        console.log('🔍 Verifying migration results...\n');
        
        // Check memory count
        const [memoryCount] = await connection.execute('SELECT COUNT(*) as count FROM memories');
        console.log(`📊 Total memories: ${memoryCount[0].count}`);
        
        // Check skip link count
        const [skipLinkCount] = await connection.execute('SELECT COUNT(*) as count FROM skip_links');
        console.log(`🔗 Total skip links: ${skipLinkCount[0].count}`);
        
        // Show sample memories
        console.log('\n📝 Sample memories:');
        const [memories] = await connection.execute('SELECT id, LEFT(content, 100) as content_preview, amplitude, layer_depth FROM memories LIMIT 5');
        console.table(memories);
        
        // Show skip links
        console.log('\n🔗 Skip links:');
        const [skipLinks] = await connection.execute('SELECT * FROM skip_links');
        console.table(skipLinks);
        
        // Show metadata
        console.log('\n📋 Metadata:');
        const [metadata] = await connection.execute('SELECT * FROM metadata');
        console.table(metadata);
        
        await connection.end();
        
        return { 
            memories: memoryCount[0].count, 
            skip_links: skipLinkCount[0].count 
        };
    } catch (error) {
        console.error('❌ Verification failed:', error.message);
    }
}

if (require.main === module) {
    verifyMigration().then(result => {
        if (result) {
            console.log('\n✅ Verification completed successfully!');
        }
    });
}

module.exports = { verifyMigration };