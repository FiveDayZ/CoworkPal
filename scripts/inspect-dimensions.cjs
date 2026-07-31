const fs = require('fs');
const path = require('path');

const dir = 'c:\\My\\Workplace\\Coding\\CoworkPal\\src\\assets\\pets\\avatars';
const files = fs.readdirSync(dir);

files.forEach(file => {
  if (file.endsWith('.png')) {
    const filePath = path.join(dir, file);
    const buf = fs.readFileSync(filePath);
    
    // Parse JPEG header
    if (buf[0] === 0xFF && buf[1] === 0xD8) {
      let offset = 2;
      let width = 0;
      let height = 0;
      while (offset < buf.length) {
        if (buf[offset] === 0xFF) {
          const marker = buf[offset + 1];
          // SOF0 (Start of Frame 0) marker is 0xC0
          // SOF2 (Progressive Start of Frame) marker is 0xC2
          if (marker === 0xC0 || marker === 0xC2) {
            // Length of segment is at offset + 2 (2 bytes)
            // Precision is at offset + 4 (1 byte)
            // Height is at offset + 5 (2 bytes)
            // Width is at offset + 7 (2 bytes)
            height = buf.readUInt16BE(offset + 5);
            width = buf.readUInt16BE(offset + 7);
            break;
          }
          // Move past marker and its length
          const len = buf.readUInt16BE(offset + 2);
          offset += 2 + len;
        } else {
          offset++;
        }
      }
      console.log(`${file}: JPEG ${width}x${height}`);
    } else {
      console.log(`${file}: Not a JPEG`);
    }
  }
});
