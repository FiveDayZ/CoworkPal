import os
from PIL import Image

avatars_dir = r"c:\My\Workplace\Coding\CoworkPal\src\assets\pets\avatars"
files = os.listdir(avatars_dir)

print("Starting avatar resizing and compression process...")

for file in files:
    if file.endswith(".png"):
        file_path = os.path.join(avatars_dir, file)
        try:
            with Image.open(file_path) as img:
                w, h = img.size
                print(f"Processing {file}: original size {w}x{h}")
                
                # Resize using LANCZOS to downscale cleanly
                resized_img = img.resize((180, 225), Image.Resampling.LANCZOS)
                
                # Save with maximum compression and optimize flag
                resized_img.save(file_path, format="PNG", optimize=True, compress_level=9)
                
                new_size = os.path.getsize(file_path)
                print(f"Saved {file} as 180x225 PNG. New size: {new_size / 1024:.2f} KB")
                
        except Exception as e:
            print(f"Error processing {file}: {e}")

print("Avatar compression complete!")
