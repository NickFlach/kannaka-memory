const fs = require('fs');
const mysql = require('mysql2/promise');

async function main() {
  // Parse export
  let str = fs.readFileSync('C:\\Users\\nickf\\Source\\kannaka-memory\\tools\\full-export.json', 'utf8');
  if (str.charCodeAt(0) === 0xFEFF) str = str.slice(1);
  const memories = JSON.parse(str);
  console.log(`Loaded ${memories.length} memories`);

  const conn = await mysql.createConnection({ host: '127.0.0.1', port: 3307, user: 'root', database: 'dolt-memory' });
  
  // Update memories with vectors
  let updated = 0, skipLinks = 0;
  for (const m of memories) {
    const vectorJson = m.vector ? JSON.stringify(m.vector) : '[]';
    const xiJson = m.xi_signature ? JSON.stringify(m.xi_signature) : '[]';
    const geomJson = m.geometry ? JSON.stringify(m.geometry) : '{}';
    
    await conn.execute(
      `UPDATE memories SET vector_data = ?, xi_signature = ?, geometry = ?, amplitude = ?, frequency = ?, phase = ?, decay_rate = ?, layer_depth = ?, hallucinated = ? WHERE id = ?`,
      [vectorJson, xiJson, geomJson, m.amplitude, m.frequency, m.phase, m.decay_rate, m.layer_depth || 0, m.hallucinated ? 1 : 0, m.id]
    );
    updated++;
    
    // Insert skip links from connections
    if (m.connections && Array.isArray(m.connections)) {
      for (const c of m.connections) {
        try {
          await conn.execute(
            `INSERT IGNORE INTO skip_links (source_id, target_id, weight, link_type, created_at) VALUES (?, ?, ?, ?, NOW())`,
            [m.id, c.target_id || c.target, c.weight || 1.0, c.link_type || c.relation_type || 'related']
          );
          skipLinks++;
        } catch(e) { /* skip dupes */ }
      }
    }
    
    if (updated % 25 === 0) console.log(`  ${updated}/${memories.length} memories, ${skipLinks} links`);
  }
  
  console.log(`Done: ${updated} memories updated, ${skipLinks} skip links inserted`);
  
  // Verify
  const [rows] = await conn.execute(`SELECT COUNT(*) as c FROM memories WHERE vector_data != '[]'`);
  console.log(`Memories with vectors: ${rows[0].c}`);
  const [links] = await conn.execute(`SELECT COUNT(*) as c FROM skip_links`);
  console.log(`Total skip links: ${links[0].c}`);
  
  await conn.end();
}

main().catch(e => { console.error(e); process.exit(1); });
