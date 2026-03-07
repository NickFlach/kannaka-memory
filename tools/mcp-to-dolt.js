#!/usr/bin/env node
/**
 * MCP-to-Dolt Migration
 * 
 * Reads all memories from the live kannaka.bin using a small Rust helper,
 * then inserts into Dolt. Since the MCP tools only return summaries,
 * we use the kannaka-migrate binary which reads the store directly.
 * 
 * Fallback: reads from a JSON dump file.
 */
'use strict';

const { execSync, spawn } = require('child_process');
const mysql = require('mysql2/promise');
const path = require('path');
const fs = require('fs');

const DOLT_DB_PATH = process.env.DOLT_DB_PATH || path.join(process.env.USERPROFILE, '.kannaka', 'dolt-memory');
const DOLT_HOST = '127.0.0.1';
const DOLT_PORT = 3307;
const DOLT_DB = 'kannaka_memory';

async function waitForServer(pool, maxWait = 30000) {
    const start = Date.now();
    while (Date.now() - start < maxWait) {
        try {
            await pool.query('SELECT 1');
            return true;
        } catch { await new Promise(r => setTimeout(r, 1000)); }
    }
    return false;
}

async function main() {
    console.log('🧠 MCP-to-Dolt Memory Migration');
    console.log('================================\n');

    // Start Dolt server
    console.log('Starting Dolt SQL server...');
    const doltProc = spawn('dolt', ['sql-server', '--port', String(DOLT_PORT)], {
        cwd: DOLT_DB_PATH,
        stdio: ['ignore', 'pipe', 'pipe'],
        env: { ...process.env, PATH: getFullPath() }
    });
    
    const pool = mysql.createPool({
        host: DOLT_HOST, port: DOLT_PORT, database: DOLT_DB,
        user: 'root', password: '', waitForConnections: true
    });

    if (!await waitForServer(pool)) {
        console.error('❌ Dolt server did not start');
        process.exit(1);
    }
    console.log('✅ Dolt server ready\n');

    // Ensure schema
    await pool.query(`
        CREATE TABLE IF NOT EXISTS memories (
            id VARCHAR(36) PRIMARY KEY,
            content LONGTEXT NOT NULL,
            amplitude DOUBLE NOT NULL,
            frequency DOUBLE NOT NULL,
            phase DOUBLE NOT NULL,
            decay_rate DOUBLE NOT NULL,
            created_at DATETIME NOT NULL,
            layer_depth TINYINT UNSIGNED NOT NULL,
            hallucinated BOOLEAN DEFAULT FALSE,
            parents LONGTEXT,
            vector_data LONGTEXT NOT NULL,
            xi_signature LONGTEXT,
            geometry LONGTEXT
        )
    `);
    await pool.query(`
        CREATE TABLE IF NOT EXISTS skip_links (
            source_id VARCHAR(36) NOT NULL,
            target_id VARCHAR(36) NOT NULL,
            weight DOUBLE NOT NULL,
            link_type VARCHAR(50) DEFAULT 'temporal',
            PRIMARY KEY (source_id, target_id)
        )
    `);

    // Get existing IDs in Dolt
    const [existing] = await pool.query('SELECT id FROM memories');
    const existingIds = new Set(existing.map(r => r.id));
    console.log(`📊 Existing in Dolt: ${existingIds.size} memories`);

    // Try kannaka-migrate binary
    const migrateBin = path.join(__dirname, '..', 'target', 'release', 'kannaka-migrate.exe');
    const dataDirs = [
        path.join(process.env.USERPROFILE, '.openclaw', 'kannaka-data'),
        path.join(process.env.USERPROFILE, '.openclaw', 'workspace', '.kannaka'),
        path.join(__dirname, '..', '.kannaka'),
    ];

    let memories = [];
    
    for (const dataDir of dataDirs) {
        const binPath = path.join(dataDir, 'kannaka.bin');
        if (!fs.existsSync(binPath)) continue;
        
        console.log(`\n🔍 Trying ${binPath} (${(fs.statSync(binPath).size / 1024 / 1024).toFixed(1)} MB)...`);
        
        try {
            // Try the migrate binary with --dump-json
            const output = execSync(
                `"${migrateBin}" --data-dir "${dataDir}" --dump-json`,
                { encoding: 'utf-8', maxBuffer: 100 * 1024 * 1024, timeout: 30000 }
            );
            const parsed = JSON.parse(output);
            const mems = Array.isArray(parsed) ? parsed : (parsed.memories || []);
            console.log(`   ✅ Extracted ${mems.length} memories`);
            memories.push(...mems);
        } catch (e) {
            console.log(`   ⚠️ Binary extract failed: ${e.message?.split('\n')[0]}`);
            
            // Try direct bincode read via Node (last resort)
            console.log('   Skipping this data dir...');
        }
    }

    if (memories.length === 0) {
        console.log('\n⚠️ No memories extracted from any data dir.');
        console.log('The bincode format may need a V3 migration in persistence.rs.');
        console.log(`\nDolt still has ${existingIds.size} memories from previous migrations.`);
        
        // Show Dolt stats
        const [counts] = await pool.query('SELECT COUNT(*) as c FROM memories');
        const [linkCounts] = await pool.query('SELECT COUNT(*) as c FROM skip_links');
        console.log(`📊 Dolt: ${counts[0].c} memories, ${linkCounts[0].c} skip links`);
    } else {
        // Filter to new memories only
        const newMems = memories.filter(m => !existingIds.has(m.id));
        console.log(`\n📝 ${memories.length} total, ${newMems.length} new to migrate`);

        let migrated = 0;
        for (const mem of newMems) {
            try {
                await pool.query(
                    `INSERT INTO memories (id, content, amplitude, frequency, phase, decay_rate, created_at, layer_depth, hallucinated, parents, vector_data, xi_signature, geometry)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                     ON DUPLICATE KEY UPDATE amplitude=VALUES(amplitude), frequency=VALUES(frequency), phase=VALUES(phase)`,
                    [
                        mem.id,
                        mem.content || '',
                        mem.amplitude || 1.0,
                        mem.frequency || 0.1,
                        mem.phase || 0,
                        mem.decay_rate || 0.001,
                        mem.created_at ? new Date(mem.created_at) : new Date(),
                        mem.layer_depth || 0,
                        mem.hallucinated || false,
                        JSON.stringify(mem.parents || []),
                        JSON.stringify(mem.vector?.slice(0, 100) || []),  // truncate vectors for storage
                        JSON.stringify(mem.xi_signature?.slice(0, 100) || []),
                        JSON.stringify(mem.geometry || null)
                    ]
                );
                migrated++;
                if (migrated % 50 === 0) process.stdout.write(`   ${migrated}/${newMems.length}\n`);
            } catch (e) {
                console.warn(`   ⚠️ Failed to insert ${mem.id}: ${e.message}`);
            }
        }

        // Insert skip links
        let links = 0;
        for (const mem of newMems) {
            for (const conn of (mem.connections || [])) {
                try {
                    await pool.query(
                        `INSERT IGNORE INTO skip_links (source_id, target_id, weight, link_type)
                         VALUES (?, ?, ?, ?)`,
                        [mem.id, conn.target_id || conn.target, conn.weight || conn.strength || 1.0, conn.link_type || 'temporal']
                    );
                    links++;
                } catch {}
            }
        }

        console.log(`\n✅ Migrated ${migrated} memories, ${links} skip links`);

        // Commit
        try {
            await pool.query("CALL DOLT_COMMIT('-Am', 'MCP migration: " + migrated + " memories')");
            console.log('💾 Committed to Dolt');
        } catch (e) {
            console.log('ℹ️  ' + e.message);
        }
    }

    // Final stats
    const [finalCount] = await pool.query('SELECT COUNT(*) as c FROM memories');
    const [finalLinks] = await pool.query('SELECT COUNT(*) as c FROM skip_links');
    console.log(`\n📊 Final Dolt state: ${finalCount[0].c} memories, ${finalLinks[0].c} skip links`);

    await pool.end();
    doltProc.kill();
    console.log('🛑 Done.');
}

function getFullPath() {
    return [
        process.env.Path || process.env.PATH || '',
        'C:\\Program Files\\Dolt\\bin',
        'C:\\Program Files (x86)\\Dolt\\bin'
    ].join(';');
}

main().catch(e => { console.error(e); process.exit(1); });
