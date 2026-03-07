const mysql = require('mysql2/promise');

async function testConnection() {
    const connectionOptions = [
        { host: 'localhost', port: 3307, user: 'root', database: 'dolt-memory' },
        { host: 'localhost', port: 3307, user: 'root', password: '', database: 'dolt-memory' },
        { host: 'localhost', port: 3307, database: 'dolt-memory' },
        { host: 'localhost', port: 3307, user: '', password: '', database: 'dolt-memory' }
    ];

    for (const options of connectionOptions) {
        try {
            console.log('Trying connection with:', JSON.stringify(options, null, 2));
            const connection = await mysql.createConnection(options);
            
            console.log('Connected successfully!');
            
            // Test query
            const [rows] = await connection.execute('SELECT COUNT(*) as count FROM memories');
            console.log('Current memory count:', rows[0].count);
            
            await connection.end();
            return; // Success, exit
        } catch (error) {
            console.error('Connection failed:', error.message);
        }
    }
    
    console.log('All connection attempts failed');
}

testConnection();