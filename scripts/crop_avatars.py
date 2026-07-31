import os
from PIL import Image

avatars_dir = r"c:\My\Workplace\Coding\CoworkPal\src\assets\pets\avatars"
files = os.listdir(avatars_dir)

print("Starting avatar crop and resize process...")

for file in files:
    if file.endswith(".png"):
        file_path = os.path.join(avatars_dir, file)
        try:
            with Image.open(file_path) as img:
                w, h = img.size
                print(f"Processing {file}: original size {w}x{h}")
                
                # Target aspect ratio is 5:4 (1.25)
                # Since the generated images are square (w = h), we crop the height to w * 4 // 5
                target_h = int(w * 4 // 5)
                top = (h - target_h) // 2
                bottom = top + target_h
                
                # Crop area: (left, top, right, bottom)
                cropped_img = img.crop((0, top, w, bottom))
                
                # Resize to target dimensions 225x180
                resized_img = cropped_img.resize((225, 180), Image.Resampling.LANCZOS)
                
                # Save back as a true PNG file to match the extension
                resized_img.save(file_path, format="PNG")
                print(f"Saved {file} as 225x180 PNG")
                
        except Exception as e:
            print(f"Error processing {file}: {e}")

print("Avatar crop and resize complete!")
