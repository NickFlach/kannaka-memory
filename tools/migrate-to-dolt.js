#!/usr/bin/env node
/**
 * Kannaka Memory Migration to Dolt  — Phase 2
 *
 * Migrates existing kannaka-memory bincode snapshots to a Dolt database.
 *
 * Configuration (environment variables or CLI flags):
 *   KANNAKA_CLI   / --cli      Path to kannaka binary  (default: auto-detect)
 *   DOLT_DB_PATH  / --db-path  Path to Dolt database dir
 *   DOLT_HOST     / --host     Dolt SQL server hostname (default: 127.0.0.1)
 *   DOLT_PORT     / --port     Dolt SQL server port     (default: 3307)
 *   DOLT_DB       / --db       Database name            (default: kannaka_memory)
 *   DOLT_USER     / --user     Database user            (default: root)
 *   DOLT_PASSWORD / --password Database password        (default: empty)
 *
 * Usage:
 *   node migrate-to-dolt.js
 *   node migrate-to-dolt.js --cli ./target/release/kannaka --db-path ~/.kannaka/dolt-memory
 *   DOLT_PORT=3308 node migrate-to-dolt.js
 *
 * Phase 2 improvements over Phase 1:
 *   - No hardcoded Windows paths — all paths from env vars or CLI args
 *   - Server readiness polling (retry up to 30s) instead of fixed 2s sleep
 *   - Idempotent upserts (ON DUPLICATE KEY UPDATE) — safe to re-run
 *   - Progress file for resumable migration of large datasets
 *   - Post-migration row count verification
 *   - datetime stored as MySQL-compatible "YYYY-MM-DD HH:MM:SS" string
 *   - Graceful Dolt server management (detect already-running server)
 */

'use strict';

const { execSync, spawn } = require('child_process');
const mysql = require('mysql2/promise');
const path = require('path');
const fs = require('fs');
const os = require('os');

// ---------------------------------------------------------------------------
// Configuration resolution (env → CLI args → defaults)
// ---------------------------------------------------------------------------

function parseArgs() {
    const args = process.argv.slice(2);
    const parsed = {};
    for (let i = 0; i < args.length; i++) {
        if (args[i].startsWith('--') && args[i + 1] && !args[i + 1].startsWith('--')) {
            parsed[args[i].slice(2)] = args[++i];
        }
    }
    return parsed;
}

function resolveKannakaCli(cliArg) {
    if (cliArg) return cliArg;
    if (process.env.KANNAKA_CLI) return process.env.KANNAKA_CLI;
    // Auto-detect: look relative to this script's location
    const scriptDir = path.dirname(__filename);
    const candidates = [
        path.join(scriptDir, '..', 'target', 'release', 'kannaka'),
        path.join(scriptDir, '..', 'target', 'release', 'kannaka.exe'),
    ];
    for (const c of candidates) {
        if (fs.existsSync(c)) return c;
    }
    return 'kannaka'; // fall back to PATH
}

function resolveConfig(cliArgs) {
    return {
        cli:      resolveKannakaCli(cliArgs['cli']),
        dbPath:   cliArgs['db-path']  || process.env.DOLT_DB_PATH  || path.join(os.homedir(), '.kannaka', 'dolt-memory'),
        host:     cliArgs['host']     || process.env.DOLT_HOST     || '127.0.0.1',
        port:     parseInt(cliArgs['port']  || process.env.DOLT_PORT || '3307', 10),
        database: cliArgs['db']       || process.env.DOLT_DB       || 'kannaka_memory',
        user:     cliArgs['user']     || process.env.DOLT_USER     || 'root',
        password: cliArgs['password'] || process.env.DOLT_PASSWORD || '',
    };
}

// ---------------------------------------------------------------------------
// Server management
// ---------------------------------------------------------------------------

async function isServerReachable(config) {
    try {
        const conn = await mysql.createConnection({
            host: config.host, port: config.port,
            database: config.database, user: config.user, password: config.password,
            connectTimeout: 1000,
        });
        await conn.end();
        return true;
    } catch {
        return false;
    }
}

async function waitForServer(config, timeoutMs = 30000) {
    const deadline = Date.now() + timeoutMs;
    let attempt = 0;
    while (Date.now() < deadline) {
        if (await isServerReachable(config)) return true;
        attempt++;
        process.stdout.write(`\r   Waiting for Dolt server... (${attempt}s)`);
        await new Promise(r => setTimeout(r, 1000));
    }
    process.stdout.write('\n');
    return false;
}

function startDoltServer(config) {
    console.log(`   Starting Dolt SQL server on port ${config.port}...`);
    const server = spawn('dolt', ['sql-server', '-H', '0.0.0.0', '-P', String(config.port)], {
        cwd: config.dbPath,
        stdio: 'pipe', // suppress output — we poll for readiness instead
    });
    server.on('error', err => console.error('❌ Failed to start Dolt server:', err.message));
    return server;
}

// ---------------------------------------------------------------------------
// Data helpers
// ---------------------------------------------------------------------------

/** Format a JS Date (or ISO string) to MySQL DATETIME string */
function toMysqlDatetime(value) {
    const d = value instanceof Date ? value : new Date(value);
    if (isNaN(d.getTime())) return new Date().toISOString().slice(0, 19).replace('T', ' ');
    return d.toISOString().slice(0, 19).replace('T', ' ');
}

/** Encode a float array as base64 Float32Array for compact storage */
function encodeVector(vector) {
    if (!vector || vector.length === 0) return null;
    const buf = Buffer.alloc(vector.length * 4);
    for (let i = 0; i < vector.length; i++) {
        buf.writeFloatLE(vector[i], i * 4);
    }
    return buf.toString('base64');
}

/** RFC-4122 v4 UUID generator (no external deps) */
function generateId() {
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
        const r = Math.random() * 16 | 0;
        return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
    });
}

// ---------------------------------------------------------------------------
// Progress file for resumable migration
// ---------------------------------------------------------------------------

function loadProgress(progressFile) {
    if (fs.existsSync(progressFile)) {
        try {
            return JSON.parse(fs.readFileSync(progressFile, 'utf-8'));
        } catch { /* ignore corrupt progress */ }
    }
    return { migratedIds: [] };
}

function saveProgress(progressFile, migratedIds) {
    fs.writeFileSync(progressFile, JSON.stringify({ migratedIds }, null, 2));
}

// ---------------------------------------------------------------------------
// Memory extraction from kannaka CLI
// ---------------------------------------------------------------------------

function extractMemories(config) {
    try {
        // Try --json flag first (newer builds), fall back to plain recall
        const output = execSync(
            `"${config.cli}" observe --json`,
            { encoding: 'utf-8', maxBuffer: 50 * 1024 * 1024 }
        );
        const report = JSON.parse(output);
        // observe --json returns a SystemReport; extract memory list if present
        if (Array.isArray(report.memories)) return report.memories;
        // Flat JSON with a top-level array
        if (Array.isArray(report)) return report;
    } catch { /* CLI may not support --json observe */ }

    // Fallback: use a tools/full-export.json if already present
    const exportFile = path.join(path.dirname(__filename), 'full-export.json');
    if (fs.existsSync(exportFile)) {
        console.log(`   Using cached export: ${exportFile}`);
        try {
            const raw = JSON.parse(fs.readFileSync(exportFile, 'utf-8'));
            if (Array.isArray(raw)) return raw;
            if (raw.memories && Array.isArray(raw.memories)) return raw.memories;
        } catch (e) {
            console.warn('⚠️ Could not parse full-export.json:', e.message);
        }
    }

    console.warn('⚠️ No memories extracted. Verify kannaka CLI path and data directory.');
    return [];
}

// ---------------------------------------------------------------------------
// Database operations
// ---------------------------------------------------------------------------

async function upsertMemory(connection, memory) {
    const id = memory.id || generateId();
    const createdAt = toMysqlDatetime(memory.created_at);

    // Idempotent upsert: safe to re-run on existing data
    await connection.execute(
        `INSERT INTO memories
            (id, content, amplitude, frequency, phase, decay_rate,
             created_at, layer_depth, hallucinated, parents, vector_data, xi_signature, geometry)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE
             content      = VALUES(content),
             amplitude    = VALUES(amplitude),
             frequency    = VALUES(frequency),
             phase        = VALUES(phase),
             decay_rate   = VALUES(decay_rate),
             layer_depth  = VALUES(layer_depth),
             hallucinated = VALUES(hallucinated),
             parents      = VALUES(parents),
             vector_data  = VALUES(vector_data),
             xi_signature = VALUES(xi_signature),
             geometry     = VALUES(geometry)`,
        [
            id,
            memory.content    || '',
            memory.amplitude  ?? 1.0,
            memory.frequency  ?? 0.1,
            memory.phase      ?? 0.0,
            memory.decay_rate ?? 0.01,
            createdAt,
            memory.layer_depth ?? 0,
            memory.hallucinated ? 1 : 0,
            memory.parents    ? JSON.stringify(memory.parents) : null,
            memory.vector     ? encodeVector(memory.vector)    : '[]',
            memory.xi_signature && memory.xi_signature.length
                              ? encodeVector(memory.xi_signature) : null,
            memory.geometry   ? JSON.stringify(memory.geometry) : null,
        ]
    );
    return id;
}

async function verifyMigration(connection, expectedCount) {
    const [[row]] = await connection.execute('SELECT COUNT(*) as cnt FROM memories');
    const actual = row.cnt;
    if (actual >= expectedCount) {
        console.log(`✅ Verification passed: ${actual} memories in Dolt (expected ≥ ${expectedCount})`);
        return true;
    }
    console.error(`❌ Verification failed: ${actual} memories in Dolt, expected ≥ ${expectedCount}`);
    return false;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
    console.log('🧠 Kannaka Memory Migration to Dolt — Phase 2');
    console.log('==============================================\n');

    const args = parseArgs();
    const config = resolveConfig(args);
    const progressFile = path.join(config.dbPath, 'migration-progress.json');

    console.log('Configuration:');
    console.log(`  CLI:      ${config.cli}`);
    console.log(`  DB path:  ${config.dbPath}`);
    console.log(`  Dolt:     ${config.host}:${config.port}/${config.database}\n`);

    // Validate DB path exists
    if (!fs.existsSync(config.dbPath)) {
        console.error(`❌ Dolt DB path not found: ${config.dbPath}`);
        console.error('   Run tools/setup-kannaka-db.js first to initialise the Dolt database.');
        process.exit(1);
    }

    // Start server if not already reachable
    let doltServer = null;
    if (await isServerReachable(config)) {
        console.log('✅ Dolt server already running — reusing existing connection\n');
    } else {
        doltServer = startDoltServer(config);
        const ready = await waitForServer(config);
        if (!ready) {
            console.error('\n❌ Dolt server did not become ready within 30 seconds.');
            if (doltServer) doltServer.kill('SIGTERM');
            process.exit(1);
        }
        console.log('\n✅ Dolt server ready\n');
    }

    let connection;
    try {
        connection = await mysql.createConnection({
            host: config.host, port: config.port,
            database: config.database, user: config.user, password: config.password,
        });

        // Load resumable progress
        const progress = loadProgress(progressFile);
        const alreadyMigrated = new Set(progress.migratedIds);
        if (alreadyMigrated.size > 0) {
            console.log(`♻️  Resuming: ${alreadyMigrated.size} IDs already migrated\n`);
        }

        // Extract memories from kannaka
        console.log('🔍 Extracting memories from kannaka...');
        const memories = extractMemories(config);
        const pending = memories.filter(m => !alreadyMigrated.has(m.id));
        console.log(`📝 ${memories.length} total, ${pending.length} pending migration\n`);

        // Upsert memories
        if (pending.length > 0) {
            console.log('📥 Upserting memories (idempotent)...');
            let successCount = 0;
            let failCount = 0;

            for (const memory of pending) {
                try {
                    const id = await upsertMemory(connection, memory);
                    alreadyMigrated.add(id);
                    successCount++;
                    if (successCount % 25 === 0 || successCount === pending.length) {
                        process.stdout.write(`\r   ✅ ${successCount}/${pending.length}`);
                        // Persist progress so we can resume on crash
                        saveProgress(progressFile, [...alreadyMigrated]);
                    }
                } catch (err) {
                    failCount++;
                    console.error(`\n❌ Memory ${memory.id}: ${err.message}`);
                }
            }

            process.stdout.write('\n');
            console.log(`\n   Inserted/updated: ${successCount}  Failed: ${failCount}`);
        }

        // Update metadata
        await connection.execute(
            'INSERT INTO metadata (key_name, value_text) VALUES (?, ?) ON DUPLICATE KEY UPDATE value_text = VALUES(value_text)',
            ['migration_date', new Date().toISOString()]
        );
        await connection.execute(
            'INSERT INTO metadata (key_name, value_text) VALUES (?, ?) ON DUPLICATE KEY UPDATE value_text = VALUES(value_text)',
            ['migrated_count', String(alreadyMigrated.size)]
        );

        // Post-migration verification
        console.log('\n🔎 Verifying migration...');
        const ok = await verifyMigration(connection, alreadyMigrated.size);

        await connection.end();

        // Commit to Dolt version control
        console.log('\n💾 Committing migration to Dolt...');
        try {
            execSync('dolt add .', { cwd: config.dbPath });
            execSync('dolt commit -m "migration: upsert from bincode store"', { cwd: config.dbPath });
            console.log('✅ Dolt commit created');
        } catch (err) {
            // Dolt commit fails if there are no changes — that's fine
            if (!err.message.includes('nothing to commit')) {
                console.warn('⚠️  Dolt commit warning:', err.message);
            } else {
                console.log('ℹ️  Nothing new to commit (already up to date)');
            }
        }

        // Clean up progress file on full success
        if (ok && fs.existsSync(progressFile)) {
            fs.unlinkSync(progressFile);
        }

        console.log(`\n🎉 Migration complete! ${alreadyMigrated.size} memories in Dolt.`);
        if (!ok) process.exit(1);

    } catch (err) {
        console.error('❌ Migration failed:', err.message);
        if (connection) { try { await connection.end(); } catch {} }
        process.exit(1);
    } finally {
        if (doltServer) {
            console.log('🛑 Stopping Dolt SQL server...');
            doltServer.kill('SIGTERM');
        }
    }
}

process.on('SIGINT',  () => { console.log('\n🛑 Interrupted'); process.exit(1); });
process.on('SIGTERM', () => { console.log('\n🛑 Terminated');  process.exit(1); });

if (require.main === module) {
    main().catch(err => { console.error(err); process.exit(1); });
}