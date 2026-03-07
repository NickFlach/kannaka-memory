const fs = require('fs');
const buf = fs.readFileSync('C:\\Users\\nickf\\Source\\kannaka-memory\\tools\\full-export.json');
console.log('First 10 bytes:', Array.from(buf.slice(0,10)).map(b => b.toString(16).padStart(2,'0')).join(' '));
console.log('Size:', buf.length);

// Strip BOM if present
let str = buf.toString('utf8');
if (str.charCodeAt(0) === 0xFEFF) {
  str = str.slice(1);
  console.log('Stripped BOM');
}

try {
  const data = JSON.parse(str);
  if (Array.isArray(data)) {
    console.log('Parsed array of', data.length, 'items');
    const first = data[0];
    console.log('First item keys:', Object.keys(first));
    console.log('vector_data length:', first.vector_data ? (Array.isArray(first.vector_data) ? first.vector_data.length : typeof first.vector_data) : 'missing');
  } else {
    console.log('Parsed object, keys:', Object.keys(data));
  }
} catch(e) {
  console.log('Parse error:', e.message.slice(0, 200));
  // Find the problematic position
  const pos = parseInt(e.message.match(/position (\d+)/)?.[1]);
  if (pos) {
    console.log('Around position', pos, ':', JSON.stringify(str.slice(Math.max(0,pos-20), pos+20)));
  }
}
