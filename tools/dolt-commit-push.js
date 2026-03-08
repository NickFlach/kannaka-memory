const mysql = require('mysql2/promise');

async function main() {
  const conn = await mysql.createConnection({
    host: '127.0.0.1', port: 3307, user: 'root', database: 'kannaka_memory'
  });
  
  const [counts] = await conn.query('SELECT COUNT(*) as cnt FROM memories');
  console.log(`Memories in Dolt: ${counts[0].cnt}`);
  
  const [links] = await conn.query('SELECT COUNT(*) as cnt FROM skip_links');
  console.log(`Skip links in Dolt: ${links[0].cnt}`);
  
  // Stage all changes
  await conn.query('CALL DOLT_ADD("-A")');
  
  // Check if there's anything to commit
  const [status] = await conn.query('SELECT COUNT(*) as cnt FROM dolt_status');
  if (status[0].cnt === 0) {
    console.log('Nothing to commit - working tree clean');
    await conn.end();
    return;
  }
  
  console.log(`${status[0].cnt} table(s) with changes`);
  
  // Commit
  const msg = `migration: ${counts[0].cnt} memories, ${links[0].cnt} skip links from full bincode export`;
  const [result] = await conn.query('CALL DOLT_COMMIT("-m", ?)', [msg]);
  console.log('Committed:', result[0].hash);
  
  await conn.end();
}

main().catch(e => { console.error(e.message); process.exit(1); });
