const mysql = require('mysql2/promise');

async function clearData() {
    try {
        const connection = await mysql.createConnection({
            host: 'localhost',
            port: 3307,
            user: 'root',
            database: 'dolt-memory'
        });
        
        console.log('Connected to Dolt database');
        
        // Clear existing data
        await connection.execute('DELETE FROM memories');
        console.log('Deleted all memories');
        
        await connection.execute('DELETE FROM skip_links');
        console.log('Deleted all skip_links');
        
        // Verify deletion
        const [memoryRows] = await connection.execute('SELECT COUNT(*) as count FROM memories');
        const [skipLinkRows] = await connection.execute('SELECT COUNT(*) as count FROM skip_links');
        
        console.log('Memory count after deletion:', memoryRows[0].count);
        console.log('Skip links count after deletion:', skipLinkRows[0].count);
        
        await connection.end();
    } catch (error) {
        console.error('Error:', error.message);
    }
}

clearData();