const fs = require('fs');
const path = require('path');

const srcDir = 'C:\\Users\\WU\\.gemini\\antigravity-ide\\brain\\f6596e85-1648-48a3-a4f0-b0796bb79f35';
const destDir = 'c:\\My\\Workplace\\Coding\\CoworkPal\\src\\assets\\pets\\avatars';

// Mapping from generated filename prefixes to target names
const mapping = {
  'deep_focus_avatar': 'deepFocus.png',
  'build_burst_avatar': 'buildBurst.png',
  'archive_flow_avatar': 'archiveFlow.png',
  'pressure_repair_avatar': 'pressureRepair.png',
  'stable_maintenance_avatar': 'stableMaintenance.png',
  'fragmented_switching_avatar': 'fragmentedSwitching.png',
  'low_load_companion_avatar': 'lowLoadCompanion.png',
  'unknown_avatar': 'unknown.png'
};

// Ensure destDir exists
if (!fs.existsSync(destDir)) {
  fs.mkdirSync(destDir, { recursive: true });
}

const files = fs.readdirSync(srcDir);

Object.entries(mapping).forEach(([prefix, targetName]) => {
  // Find the generated file matching the prefix and having the latest timestamp (usually the only one)
  const matchedFiles = files.filter(f => f.startsWith(prefix) && f.endsWith('.png'));
  if (matchedFiles.length === 0) {
    console.error(`Error: Could not find generated file for prefix "${prefix}"`);
    return;
  }
  
  // Sort by filename to find the newest timestamp one (in case there are multiple)
  matchedFiles.sort();
  const newestFile = matchedFiles[matchedFiles.length - 1];
  
  const srcPath = path.join(srcDir, newestFile);
  const destPath = path.join(destDir, targetName);
  
  console.log(`Copying ${newestFile} -> ${targetName}`);
  fs.copyFileSync(srcPath, destPath);
});

console.log('All avatar files copied successfully!');
