// Fix 148-dim visual memory vectors by removing them from the data files
// The memories will be re-seen with proper 10,000-dim vectors after
const fs = require('fs');
const path = require('path');

const dataDir = process.env.KANNAKA_DATA_DIR || path.join(require('os').homedir(), '.openclaw', 'kannaka-data');
const memoriesFile = path.join(dataDir, 'memories.json');

if (!fs.existsSync(memoriesFile)) {
  // Try bincode - but that's binary. Let's check what format is actually used.
  console.log('No memories.json found at', memoriesFile);
  console.log('Contents of', dataDir, ':');
  if (fs.existsSync(dataDir)) {
    fs.readdirSync(dataDir).forEach(f => {
      const stat = fs.statSync(path.join(dataDir, f));
      console.log(`  ${f} (${stat.size} bytes)`);
    });
  }
  process.exit(1);
}
