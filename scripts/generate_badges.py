import os
import re
import math
from PIL import Image, ImageDraw

SPEC_PATH = r"c:\My\Workplace\Coding\CoworkPal\.docs\achievement_badge_system_development_spec.md"
OUTPUT_DIR = r"c:\My\Workplace\Coding\CoworkPal\src\assets\achievements"

# Create output dir if not exist
os.makedirs(OUTPUT_DIR, exist_ok=True)

# Custom 3x5 Pixel Font
PIXEL_FONT = {
    '0': [
        (0,0), (1,0), (2,0),
        (0,1),         (2,1),
        (0,2),         (2,2),
        (0,3),         (2,3),
        (0,4), (1,4), (2,4)
    ],
    '1': [
               (1,0),
               (1,1),
               (1,2),
               (1,3),
               (1,4)
    ],
    '2': [
        (0,0), (1,0), (2,0),
                       (2,1),
        (0,2), (1,2), (2,2),
        (0,3),
        (0,4), (1,4), (2,4)
    ],
    '3': [
        (0,0), (1,0), (2,0),
                       (2,1),
        (0,2), (1,2), (2,2),
                       (2,3),
        (0,4), (1,4), (2,4)
    ],
    '4': [
        (0,0),         (2,0),
        (0,1),         (2,1),
        (0,2), (1,2), (2,2),
                       (2,3),
                       (2,4)
    ],
    '5': [
        (0,0), (1,0), (2,0),
        (0,1),
        (0,2), (1,2), (2,2),
                       (2,3),
        (0,4), (1,4), (2,4)
    ],
    '6': [
        (0,0), (1,0), (2,0),
        (0,1),
        (0,2), (1,2), (2,2),
        (0,3),         (2,3),
        (0,4), (1,4), (2,4)
    ],
    '7': [
        (0,0), (1,0), (2,0),
                       (2,1),
                       (2,2),
                       (2,3),
                       (2,4)
    ],
    '8': [
        (0,0), (1,0), (2,0),
        (0,1),         (2,1),
        (0,2), (1,2), (2,2),
        (0,3),         (2,3),
        (0,4), (1,4), (2,4)
    ],
    '9': [
        (0,0), (1,0), (2,0),
        (0,1),         (2,1),
        (0,2), (1,2), (2,2),
                       (2,3),
        (0,4), (1,4), (2,4)
    ],
    'K': [
        (0,0),         (2,0),
        (0,1), (1,1),
        (0,2),
        (0,3), (1,3),
        (0,4),         (2,4)
    ],
    'M': [
        (0,0),                 (4,0),
        (0,1), (1,1),   (3,1), (4,1),
        (0,2),     (2,2),      (4,2),
        (0,3),                 (4,3),
        (0,4),                 (4,4)
    ], # 5x5
    'G': [
        (0,0), (1,0), (2,0),
        (0,1),
        (0,2),         (2,2),
        (0,3),         (2,3),
        (0,4), (1,4), (2,4)
    ],
    'T': [
        (0,0), (1,0), (2,0),
               (1,1),
               (1,2),
               (1,3),
               (1,4)
    ],
    'D': [
        (0,0), (1,0),
        (0,1),         (2,1),
        (0,2),         (2,2),
        (0,3),         (2,3),
        (0,4), (1,4)
    ],
    'H': [
        (0,0),         (2,0),
        (0,1),         (2,1),
        (0,2), (1,2), (2,2),
        (0,3),         (2,3),
        (0,4),         (2,4)
    ],
    'd': [
                       (2,0),
                       (2,1),
        (0,2),         (2,2),
        (0,3),         (2,3),
        (0,4), (1,4), (2,4)
    ],
    'm': [
        (0,0),         (2,0),         (4,0),
        (0,1), (1,1), (2,1), (3,1), (4,1),
        (0,2),         (2,2),         (4,2),
        (0,3),         (2,3),         (4,3),
        (0,4),         (2,4),         (4,4)
    ], # 5x5
    '.': [
        (0,4)
    ],
    '+': [
               (1,1),
        (0,2), (1,2), (2,2),
               (1,3)
    ],
    'A': [
        (0,0), (1,0), (2,0),
        (0,1),         (2,1),
        (0,2), (1,2), (2,2),
        (0,3),         (2,3),
        (0,4),         (2,4)
    ],
    'B': [
        (0,0), (1,0),
        (0,1),         (2,1),
        (0,2), (1,2),
        (0,3),         (2,3),
        (0,4), (1,4)
    ],
    'S': [
        (0,0), (1,0), (2,0),
        (0,1),
        (0,2), (1,2), (2,2),
                       (2,3),
        (0,4), (1,4), (2,4)
    ],
    'L': [
        (0,0),
        (0,1),
        (0,2),
        (0,3),
        (0,4), (1,4), (2,4)
    ],
    'V': [
        (0,0),         (2,0),
        (0,1),         (2,1),
        (0,2),         (2,2),
        (0,3),         (2,3),
               (1,4)
    ]
}

def draw_text_pixel(draw, text, x_start, y_start, color):
    """Draw text pixel by pixel using our custom 3x5 font."""
    cur_x = x_start
    for char in text:
        if char == ' ':
            cur_x += 3
            continue
        char_data = PIXEL_FONT.get(char)
        if char_data:
            width = 3
            if char in ('M', 'm'):
                width = 5
            for px, py in char_data:
                draw.point((cur_x + px, y_start + py), fill=color)
            cur_x += width + 1
        else:
            cur_x += 2 # default space for unknown characters

# Achievement mapping to Symbol and Label
ACHIEVEMENT_DESIGNS = {
    # Entry (A001 - A020)
    'A001': ('cat_awake', ''),
    'A002': ('clock', '30m'),
    'A003': ('report', '1'),
    'A004': ('terminal', 'DASH'),
    'A005': ('parts', 'SHOP'),
    'A006': ('terminal', 'DEV'),
    'A007': ('settings', 'SET'),
    'A008': ('terminal', 'BAR'),
    'A009': ('terminal', 'TRAY'),
    'A010': ('cat_talk', ''),
    'A011': ('cat_pet', '1'),
    'A012': ('cat_drag', '1'),
    'A013': ('parts', 'BOLT'),
    'A014': ('level_up', '1'),
    'A015': ('coin', '100'),
    'A016': ('lightbulb', '10'),
    'A017': ('keyboard', '100'),
    'A018': ('mouse', '50'),
    'A019': ('share', 'REP'),
    'A020': ('share', 'GALL'),

    # Normal (A021 - A040)
    'A021': ('clock', '4H'),
    'A022': ('calendar', '3d'),
    'A023': ('flame', '3d'),
    'A024': ('report', '3'),
    'A025': ('report', '3d'),
    'A026': ('explore', 'ALL'),
    'A027': ('cat_pet', '30'),
    'A028': ('cat_talk', '20'),
    'A029': ('cat_drag', '10'),
    'A030': ('settings', 'RGB'),
    'A031': ('terminal', 'ITEM'),
    'A032': ('level_up', '3'),
    'A033': ('parts', '+3'),
    'A034': ('parts', '3M'),
    'A035': ('coin', '1K'),
    'A036': ('lightbulb', '100'),
    'A037': ('thermometer', '1H'),
    'A038': ('database', '10G'),
    'A039': ('keyboard', '1.5K'),
    'A040': ('share', 'BDG'),

    # Skilled (A041 - A060)
    'A041': ('clock', '24H'),
    'A042': ('calendar', '14d'),
    'A043': ('flame', '7d'),
    'A044': ('report', '14'),
    'A045': ('shield', '10'),
    'A046': ('calendar', '4T'),
    'A047': ('level_up', '10'),
    'A048': ('parts', 'ALL5'),
    'A049': ('parts', 'PRC5'),
    'A050': ('level_up', '20'),
    'A051': ('coin', '10K'),
    'A052': ('lightbulb', '1K'),
    'A053': ('terminal', '10H'),
    'A054': ('database', '5H'),
    'A055': ('terminal', '3H'),
    'A056': ('database', '100G'),
    'A057': ('network', '50G'),
    'A058': ('cat_awake', '500'),
    'A059': ('settings', 'SET5'),
    'A060': ('share', '5'),

    # Elite (A061 - A080)
    'A061': ('clock', '7D'),
    'A062': ('calendar', '60d'),
    'A063': ('flame', '30d'),
    'A064': ('report', '60'),
    'A065': ('shield', '30'),
    'A066': ('lightbulb', 'FOC'),
    'A067': ('parts', 'BST'),
    'A068': ('level_up', '30'),
    'A069': ('parts', 'A20'),
    'A070': ('parts', 'P20'),
    'A071': ('level_up', '50'),
    'A072': ('coin', '100K'),
    'A073': ('lightbulb', '10K'),
    'A074': ('shield', 'SAFE'),
    'A075': ('snow', 'COOL'),
    'A076': ('report', '100'),
    'A077': ('share', '20'),
    'A078': ('share', 'IN5'),
    'A079': ('hidden', 'NITE'),
    'A080': ('hidden', 'TRPL'),

    # Epic (A081 - A100)
    'A081': ('clock', '30D'),
    'A082': ('calendar', '180d'),
    'A083': ('flame', '90d'),
    'A084': ('report', '180'),
    'A085': ('shield', '100'),
    'A086': ('lightbulb', 'FOC50'),
    'A087': ('parts', 'REP25'),
    'A088': ('level_up', '60'),
    'A089': ('parts', 'A50'),
    'A090': ('parts', 'P50'),
    'A091': ('level_up', 'MAX'),
    'A092': ('coin', '1M'),
    'A093': ('lightbulb', '100K'),
    'A094': ('database', '5T'),
    'A095': ('network', '1T'),
    'A096': ('keyboard', '1M'),
    'A097': ('snow', 'COOL60'),
    'A098': ('shield', '80'),
    'A099': ('explore', 'ALL9'),
    'A100': ('hidden', 'LOWP'),

    # Legendary (A101 - A120)
    'A101': ('clock', '365d'),
    'A102': ('calendar', '365d'),
    'A103': ('flame', '180d'),
    'A104': ('report', '365'),
    'A105': ('shield', '180'),
    'A106': ('explore', 'ALL7'),
    'A107': ('level_up', '100'),
    'A108': ('level_up', 'ALLM'),
    'A109': ('coin', '10M'),
    'A110': ('lightbulb', '1M'),
    'A111': ('database', '50T'),
    'A112': ('keyboard', '10M'),
    'A113': ('shield', '365d'),
    'A114': ('explore', '100'),
    'A115': ('calendar', '12M'),
    'A116': ('cat_awake', 'ALL'),
    'A117': ('hidden', 'MIDN'),
    'A118': ('hidden', '404'),
    'A119': ('hidden', 'ALL6'),
    'A120': ('hidden', 'STAR'),

    # Worklog Rarity (A121-A126)
    'A121': ('worklog_card', 'B'),
    'A122': ('worklog_card', 'A'),
    'A123': ('worklog_card', 'S'),
    'A124': ('worklog_card', 'SS'),
    'A125': ('cards_fan', '7'),
    'A126': ('cards_stack', '30'),

    # Worklog Title (A127-A132)
    'A127': ('pressure_gauge', 'LV2'),
    'A128': ('pressure_gauge', 'LV3'),
    'A129': ('rank_badge', 'LV3'),
    'A130': ('rank_badge', 'LV5'),
    'A131': ('crown_all', 'LV3'),
    'A132': ('crown_all', '')
}

def parse_achievements():
    """Parse spec file to retrieve list of achievements."""
    achievements = []
    current_difficulty = None
    
    difficulty_map = {
        '### 4.1 入门（简单）': 'entry',
        '### 4.2 进阶（普通）': 'normal',
        '### 4.3 熟练（中等）': 'skilled',
        '### 4.4 精英（困难）': 'elite',
        '### 4.5 史诗': 'epic',
        '### 4.6 传说': 'legendary'
    }

    with open(SPEC_PATH, 'r', encoding='utf-8') as f:
        lines = f.readlines()
        
    for line in lines:
        line_strip = line.strip()
        if line_strip.startswith('### 4.'):
            for k, v in difficulty_map.items():
                if k in line_strip:
                    current_difficulty = v
                    break
            continue
            
        if line_strip.startswith('| A'):
            # Table row
            parts = [p.strip() for p in line_strip.split('|') if p.strip()]
            if len(parts) >= 6:
                id_ = parts[0]
                category = parts[1]
                name = parts[2]
                badge_key_raw = parts[5]
                badge_key = badge_key_raw.replace('`', '')
                achievements.append({
                    'id': id_,
                    'category': category,
                    'name': name,
                    'difficulty': current_difficulty,
                    'badge_key': badge_key
                })
    return achievements

# Difficulties definitions
DIFFICULTY_THEMES = {
    'entry': {
        'outer': (92, 46, 11),       # Dark Bronze
        'main': (205, 127, 50),     # Bronze
        'light': (230, 161, 92),     # Light Bronze Highlight
        'shadow': (139, 69, 19),     # SaddleBrown Shadow
        'text': (245, 222, 179)      # Wheat text
    },
    'normal': {
        'outer': (0, 77, 64),        # Dark Teal
        'main': (32, 178, 170),      # Light Sea Green
        'light': (128, 242, 230),    # Bright Cyan Highlight
        'shadow': (0, 121, 107),     # Teal Shadow
        'text': (224, 255, 255)      # Light Cyan text
    },
    'skilled': {
        'outer': (10, 25, 47),       # Dark Slate Blue
        'main': (30, 144, 255),      # Dodger Blue
        'light': (128, 191, 255),    # Soft Blue Highlight
        'shadow': (0, 86, 179),      # Royal Blue Shadow
        'text': (240, 248, 255)      # Alice Blue text
    },
    'elite': {
        'outer': (42, 8, 92),        # Dark Purple
        'main': (153, 50, 204),      # Dark Orchid
        'light': (216, 150, 255),    # Soft Violet Highlight
        'shadow': (106, 13, 173),     # Purple Shadow
        'text': (250, 240, 255)      # Lavender text
    },
    'epic': {
        'outer': (92, 67, 8),        # Dark Goldenrod
        'main': (255, 215, 0),       # Gold
        'light': (255, 230, 128),    # Shimmering Gold Highlight
        'shadow': (184, 134, 11),    # Dark Goldenrod Shadow
        'text': (255, 250, 205)      # Lemon Chiffon text
    },
    'legendary': {
        # Rainbow Border special values
        'outer': (20, 20, 20),
        'main': (255, 215, 0),       # Will be animated/rainbowed in code
        'light': (255, 255, 255),
        'shadow': (50, 50, 50),
        'text': (255, 255, 255)
    }
}

def get_rainbow_color(x, y):
    """Generate a rainbow color for coordinates x, y relative to center (32, 32)."""
    angle = math.atan2(y - 32, x - 32)
    hue = (angle + math.pi) / (2 * math.pi) # 0.0 to 1.0
    
    # Simple hue to RGB
    r, g, b = 0, 0, 0
    h_six = hue * 6.0
    i = int(h_six)
    f = h_six - i
    q = int(255 * (1.0 - f))
    t = int(255 * f)
    
    if i == 0:   r, g, b = 255, t, 0
    elif i == 1: r, g, b = q, 255, 0
    elif i == 2: r, g, b = 0, 255, t
    elif i == 3: r, g, b = 0, q, 255
    elif i == 4: r, g, b = t, 0, 255
    else:        r, g, b = 255, 0, q
    
    return (r, g, b)

def draw_frame(draw, difficulty):
    """Draw a beautiful pixel art frame shield/octagon."""
    theme = DIFFICULTY_THEMES[difficulty]
    
    # Background fill (Dark slate grey #18181b)
    for y in range(4, 60):
        for x in range(4, 60):
            # Clip corners to make octagon background
            if x + y < 20 or (63-x) + y < 20 or x + (63-y) < 20 or (63-x) + (63-y) < 20:
                continue
            # Diagonal stripe pattern background
            if (x + y) % 6 == 0:
                draw.point((x, y), fill=(28, 28, 30))
            else:
                draw.point((x, y), fill=(20, 20, 22))

    # Draw frame boundary
    # Coordinates for octagon
    points = [
        (16, 4), (47, 4), (59, 16), (59, 47),
        (47, 59), (16, 59), (4, 47), (4, 16)
    ]
    
    # Outer Border (black/dark outer)
    draw.polygon(points, outline=theme['outer'])
    
    # Draw thicker borders and highlight pixels manually
    def draw_pixel_line(p1, p2, fill):
        # Bresenham's line algorithm
        x1, y1 = p1
        x2, y2 = p2
        dx = abs(x2 - x1)
        dy = abs(y2 - y1)
        sx = 1 if x1 < x2 else -1
        sy = 1 if y1 < y2 else -1
        err = dx - dy
        
        while True:
            draw.point((x1, y1), fill=fill)
            if x1 == x2 and y1 == y2:
                break
            e2 = 2 * err
            if e2 > -dy:
                err -= dy
                x1 += sx
            if e2 < dx:
                err += dx
                y1 += sy

    # Draw inner border with theme main colors
    inner_points = [
        (16, 5), (47, 5), (58, 16), (58, 47),
        (47, 58), (16, 58), (5, 47), (5, 16)
    ]
    for i in range(len(inner_points)):
        p1 = inner_points[i]
        p2 = inner_points[(i+1)%len(inner_points)]
        
        # Legendary gets rainbow colors
        if difficulty == 'legendary':
            # Draw point by point to get gradient
            x1, y1 = p1
            x2, y2 = p2
            dx = abs(x2 - x1)
            dy = abs(y2 - y1)
            sx = 1 if x1 < x2 else -1
            sy = 1 if y1 < y2 else -1
            err = dx - dy
            while True:
                draw.point((x1, y1), fill=get_rainbow_color(x1, y1))
                # highlight ring
                draw.point((x1 + sx, y1 + sy), fill=get_rainbow_color(x1+sx, y1+sy))
                if x1 == x2 and y1 == y2:
                    break
                e2 = 2 * err
                if e2 > -dy:
                    err -= dy
                    x1 += sx
                if e2 < dx:
                    err += dx
                    y1 += sy
        else:
            # Normal shadow/highlight lines
            fill_color = theme['main']
            # Highlight top and left
            if i in (0, 7): # top, top-left
                fill_color = theme['light']
            elif i in (3, 4, 5): # bottom-right, bottom, bottom-left
                fill_color = theme['shadow']
            draw_pixel_line(p1, p2, fill=fill_color)

            # Draw second ring
            p1_2 = (inner_points[i][0] + (1 if inner_points[i][0] < 32 else -1), inner_points[i][1] + (1 if inner_points[i][1] < 32 else -1))
            p2_2 = (inner_points[(i+1)%len(inner_points)][0] + (1 if inner_points[(i+1)%len(inner_points)][0] < 32 else -1), inner_points[(i+1)%len(inner_points)][1] + (1 if inner_points[(i+1)%len(inner_points)][1] < 32 else -1))
            draw_pixel_line(p1_2, p2_2, fill=theme['shadow'] if i in (0, 7) else theme['outer'])

    # Corner decorations (mini studs)
    corners = [(16, 4), (47, 4), (59, 16), (59, 47), (47, 59), (16, 59), (4, 47), (4, 16)]
    for cx, cy in corners:
        draw.point((cx, cy), fill=(255, 255, 255) if difficulty != 'entry' else (255, 223, 128))

def draw_symbol_cat_awake(draw, theme):
    """Draw a cute pixelated cat face."""
    # Cat ears
    draw.polygon([(22, 22), (20, 16), (26, 20)], fill=(240, 150, 80)) # Left Ear
    draw.polygon([(42, 22), (44, 16), (38, 20)], fill=(240, 150, 80)) # Right Ear
    # Face body
    draw.rectangle([22, 20, 42, 38], fill=(225, 225, 225))
    # Outer cheeks
    draw.point((21, 28), fill=(225, 225, 225))
    draw.point((21, 29), fill=(225, 225, 225))
    draw.point((21, 30), fill=(225, 225, 225))
    draw.point((43, 28), fill=(225, 225, 225))
    draw.point((43, 29), fill=(225, 225, 225))
    draw.point((43, 30), fill=(225, 225, 225))
    
    # Eyes (open, sparkling blue)
    draw.point((27, 27), fill=(0, 150, 255))
    draw.point((27, 28), fill=(0, 150, 255))
    draw.point((37, 27), fill=(0, 150, 255))
    draw.point((37, 28), fill=(0, 150, 255))
    
    # Nose & mouth
    draw.point((32, 31), fill=(240, 120, 120)) # nose
    draw.point((31, 33), fill=(80, 80, 80)) # mouth left
    draw.point((33, 33), fill=(80, 80, 80)) # mouth right
    # Rosy cheeks
    draw.point((25, 31), fill=(255, 180, 180))
    draw.point((39, 31), fill=(255, 180, 180))

def draw_symbol_cat_talk(draw, theme):
    """Draw cat talking with a chat bubble."""
    # Mini cat head
    draw.rectangle([18, 26, 34, 40], fill=(225, 225, 225))
    draw.polygon([(18, 26), (16, 20), (22, 24)], fill=(240, 150, 80)) # Ear L
    draw.polygon([(34, 26), (36, 20), (30, 24)], fill=(240, 150, 80)) # Ear R
    draw.point((22, 31), fill=(80, 80, 80)) # Eye L
    draw.point((30, 31), fill=(80, 80, 80)) # Eye R
    draw.point((26, 34), fill=(240, 120, 120)) # nose
    
    # Speech bubble
    draw.rectangle([34, 15, 52, 25], fill=(255, 255, 255))
    draw.point((33, 20), fill=(255, 255, 255))
    draw.point((35, 26), fill=(255, 255, 255))
    # Dot inside bubble
    draw.point((40, 20), fill=(255, 100, 100))
    draw.point((43, 20), fill=(255, 100, 100))
    draw.point((46, 20), fill=(255, 100, 100))

def draw_symbol_cat_pet(draw, theme):
    """Draw a hand petting a cat."""
    # Cat head at bottom
    draw.rectangle([20, 28, 44, 46], fill=(225, 225, 225))
    draw.polygon([(20, 28), (18, 22), (24, 26)], fill=(240, 150, 80))
    draw.polygon([(44, 28), (46, 22), (40, 26)], fill=(240, 150, 80))
    draw.point((26, 33), fill=(100, 100, 100))
    draw.point((38, 33), fill=(100, 100, 100))
    draw.point((32, 36), fill=(240, 120, 120))
    
    # Hand/Finger petting
    # Draw pink/skin hand
    draw.rectangle([28, 16, 38, 23], fill=(255, 218, 185)) # hand
    draw.point((27, 20), fill=(255, 190, 150))
    draw.point((39, 20), fill=(255, 190, 150))
    # Heart sparkles
    draw.point((16, 20), fill=(255, 80, 80))
    draw.point((48, 20), fill=(255, 80, 80))

def draw_symbol_cat_drag(draw, theme):
    """Draw cat body moving / drag."""
    # Cat face stretching
    draw.rectangle([20, 24, 44, 40], fill=(225, 225, 225))
    draw.polygon([(20, 24), (18, 18), (24, 22)], fill=(240, 150, 80))
    draw.polygon([(44, 24), (46, 18), (40, 22)], fill=(240, 150, 80))
    draw.point((25, 30), fill=(80, 80, 80))
    draw.point((39, 30), fill=(80, 80, 80))
    # Swirly/dizzy eyes
    draw.point((24, 29), fill=(80,80,80))
    draw.point((26, 31), fill=(80,80,80))
    draw.point((38, 29), fill=(80,80,80))
    draw.point((40, 31), fill=(80,80,80))
    # Drag arrows
    draw.rectangle([14, 32, 17, 34], fill=(255, 255, 0)) # Left yellow arrow
    draw.point((13, 33), fill=(255, 255, 0))
    draw.rectangle([47, 32, 50, 34], fill=(255, 255, 0)) # Right arrow
    draw.point((51, 33), fill=(255, 255, 0))

def draw_symbol_clock(draw, theme):
    """Draw an hourglass."""
    # Hourglass frame
    draw.rectangle([22, 16, 42, 18], fill=theme['main']) # Top plate
    draw.rectangle([22, 46, 42, 48], fill=theme['main']) # Bottom plate
    # Glass bodies
    draw.line([(24, 19), (28, 31)], fill=(200, 240, 255)) # Left top
    draw.line([(40, 19), (36, 31)], fill=(200, 240, 255)) # Right top
    draw.line([(24, 45), (28, 33)], fill=(200, 240, 255)) # Left bottom
    draw.line([(40, 45), (36, 33)], fill=(200, 240, 255)) # Right bottom
    
    # Sand
    draw.rectangle([26, 20, 38, 24], fill=(255, 223, 128)) # Sand top
    draw.point((32, 28), fill=(255, 223, 128))
    draw.point((32, 31), fill=(255, 223, 128)) # flow
    draw.point((32, 34), fill=(255, 223, 128))
    draw.rectangle([28, 41, 36, 45], fill=(255, 223, 128)) # Sand bottom

def draw_symbol_report(draw, theme):
    """Draw a work report document."""
    # Paper sheet
    draw.rectangle([22, 16, 42, 46], fill=(245, 245, 245))
    # Folded corner
    draw.polygon([(38, 16), (42, 20), (38, 20)], fill=(200, 200, 200))
    draw.point((42, 16), fill=(16, 16, 18)) # Background clip
    draw.point((41, 16), fill=(16, 16, 18))
    draw.point((42, 17), fill=(16, 16, 18))
    
    # Text lines on report
    draw.line([(26, 24), (36, 24)], fill=(120, 120, 120))
    draw.line([(26, 28), (38, 28)], fill=(120, 120, 120))
    draw.line([(26, 32), (34, 32)], fill=(120, 120, 120))
    draw.line([(26, 36), (38, 36)], fill=(120, 120, 120))
    
    # Green checkmark overlay
    draw.line([(32, 42), (35, 45)], fill=(0, 220, 100), width=2)
    draw.line([(35, 45), (41, 38)], fill=(0, 220, 100), width=2)

def draw_symbol_terminal(draw, theme):
    """Draw a computer monitor with console/graphs."""
    # Screen Frame
    draw.rectangle([16, 18, 48, 40], fill=(80, 80, 80)) # bezel
    draw.rectangle([18, 20, 46, 38], fill=(10, 15, 12)) # screen active
    # Stand
    draw.rectangle([28, 41, 36, 45], fill=(120, 120, 120)) # stand neck
    draw.rectangle([24, 46, 40, 48], fill=(80, 80, 80)) # stand base
    
    # Inside screen pixel art
    # Draw green grid lines or code lines
    draw.line([(20, 24), (28, 24)], fill=(0, 255, 100))
    draw.line([(20, 28), (34, 28)], fill=(0, 255, 100))
    draw.line([(20, 32), (24, 32)], fill=(0, 255, 100))
    # Small graph bar on the right
    draw.rectangle([40, 30, 43, 36], fill=(255, 100, 0))

def draw_symbol_parts(draw, theme):
    """Draw workshop hammer & gear/screws."""
    # Hammer
    # Wooden handle
    draw.line([(20, 44), (36, 28)], fill=(139, 90, 43), width=2)
    # Metal head
    draw.rectangle([34, 22, 40, 28], fill=(180, 180, 180))
    draw.rectangle([32, 24, 42, 26], fill=(120, 120, 120))
    # Small gear or bolt decoration
    draw.arc([16, 16, 28, 28], 0, 360, fill=(200, 180, 0))
    draw.point((22, 22), fill=(100, 100, 100))

def draw_symbol_settings(draw, theme):
    """Draw mechanical gear + settings slider."""
    # Gear
    draw.arc([22, 16, 42, 36], 0, 360, fill=(200, 200, 200))
    draw.arc([25, 19, 39, 33], 0, 360, fill=(40, 40, 42))
    # Gear teeth
    draw.point((32, 14), fill=(200, 200, 200))
    draw.point((32, 38), fill=(200, 200, 200))
    draw.point((20, 26), fill=(200, 200, 200))
    draw.point((44, 26), fill=(200, 200, 200))
    draw.point((24, 18), fill=(200, 200, 200))
    draw.point((40, 34), fill=(200, 200, 200))
    draw.point((24, 34), fill=(200, 200, 200))
    draw.point((40, 18), fill=(200, 200, 200))
    
    # Slider control below
    draw.line([(18, 44), (46, 44)], fill=(100, 100, 100))
    draw.rectangle([30, 42, 34, 46], fill=(0, 255, 255)) # slider knob

def draw_symbol_keyboard(draw, theme):
    """Draw a desktop keyboard."""
    # Frame
    draw.rectangle([14, 22, 50, 42], fill=(60, 60, 60))
    draw.rectangle([16, 24, 48, 40], fill=(30, 30, 30))
    
    # Keys grid (simple dots)
    for y in (26, 30, 34, 38):
        for x in range(18, 48, 4):
            # Spacebar
            if y == 38 and 24 <= x <= 38:
                draw.rectangle([24, 38, 38, 39], fill=(220, 220, 220))
                continue
            if y == 38 and x < 24:
                continue
            if y == 38 and x > 38:
                continue
            draw.point((x, y), fill=(220, 220, 220))
            draw.point((x+1, y), fill=(220, 220, 220))

def draw_symbol_mouse(draw, theme):
    """Draw a classic computer mouse and cursor."""
    # Mouse body
    draw.rectangle([20, 24, 36, 44], fill=(240, 240, 240))
    # Rounded corners
    draw.point((20, 24), fill=(16, 16, 18))
    draw.point((36, 24), fill=(16, 16, 18))
    draw.point((20, 44), fill=(16, 16, 18))
    draw.point((36, 44), fill=(16, 16, 18))
    
    # Buttons split
    draw.line([(28, 24), (28, 32)], fill=(80, 80, 80))
    draw.line([(20, 32), (36, 32)], fill=(80, 80, 80))
    # Scroll wheel
    draw.rectangle([27, 26, 29, 30], fill=(255, 100, 0))
    
    # Cursor pointer overlay
    draw.polygon([(36, 16), (44, 24), (40, 25), (45, 30), (43, 31), (38, 26), (36, 28)], fill=(0, 255, 255))

def draw_symbol_flame(draw, theme):
    """Draw a burning hot fire flame (streak)."""
    # Flame shape outer
    # Center 32, 32
    draw.polygon([(32, 14), (20, 32), (24, 46), (40, 46), (44, 32)], fill=(255, 69, 0)) # Red-Orange
    draw.polygon([(32, 20), (24, 34), (28, 44), (36, 44), (40, 34)], fill=(255, 140, 0)) # Dark Orange
    draw.polygon([(32, 26), (27, 36), (30, 42), (34, 42), (37, 36)], fill=(255, 215, 0)) # Yellow
    draw.polygon([(32, 32), (29, 38), (32, 40), (35, 38)], fill=(255, 255, 255)) # White hot core

def draw_symbol_calendar(draw, theme):
    """Draw a calendar page."""
    # Calendar body
    draw.rectangle([18, 16, 46, 46], fill=(245, 245, 245))
    # Red header binding
    draw.rectangle([18, 16, 46, 22], fill=(230, 50, 50))
    # Calendar rings
    draw.rectangle([24, 13, 26, 17], fill=(180, 180, 180))
    draw.rectangle([38, 13, 40, 17], fill=(180, 180, 180))
    
    # Grid dates
    for y in range(26, 44, 5):
        for x in range(22, 44, 5):
            # Highlight some marked days in green/red
            if (x + y) % 3 == 0:
                draw.rectangle([x, y, x+2, y+2], fill=(0, 200, 80)) # active day
            else:
                draw.rectangle([x, y, x+2, y+2], fill=(180, 180, 180))

def draw_symbol_database(draw, theme):
    """Draw database cylindrical discs."""
    # Three stacked cylinders
    # Bottom
    draw.ellipse([20, 36, 44, 46], fill=(120, 120, 130), outline=(80, 80, 90))
    draw.rectangle([20, 32, 44, 41], fill=(120, 120, 130))
    draw.ellipse([20, 27, 44, 37], fill=(160, 160, 170), outline=(100, 100, 110))
    # Middle
    draw.rectangle([20, 24, 44, 32], fill=(120, 120, 130))
    draw.ellipse([20, 19, 44, 29], fill=(170, 170, 180), outline=(110, 110, 120))
    # Top
    draw.rectangle([20, 16, 44, 24], fill=(140, 140, 150))
    draw.ellipse([20, 12, 44, 21], fill=(200, 200, 210), outline=(150, 150, 160))
    
    # Shimmering blue data query lights
    draw.point((24, 18), fill=(0, 255, 255))
    draw.point((24, 28), fill=(0, 255, 255))
    draw.point((24, 38), fill=(0, 255, 255))

def draw_symbol_network(draw, theme):
    """Draw network nodes / ethernet ports."""
    # Draw central router
    draw.rectangle([24, 26, 40, 38], fill=(80, 80, 90))
    draw.line([(24, 32), (40, 32)], fill=(40, 40, 50))
    # Green blinking port lights
    draw.point((28, 30), fill=(0, 255, 0))
    draw.point((32, 30), fill=(0, 255, 0))
    draw.point((36, 30), fill=(0, 255, 0))
    
    # Outgoing network connections/cables
    draw.line([(32, 38), (32, 48)], fill=(150, 150, 160))
    draw.line([(20, 48), (44, 48)], fill=(150, 150, 160))
    # Cable branches
    draw.line([(20, 48), (20, 44)], fill=(150, 150, 160))
    draw.line([(44, 48), (44, 44)], fill=(150, 150, 160))

def draw_symbol_shield(draw, theme):
    """Draw defensive shield crest."""
    # Shield shape points
    # (32, 16) top center -> (46, 16) top right -> (46, 34) mid right -> (32, 48) bottom center -> (18, 34) mid left -> (18, 16) top left
    draw.polygon([(32, 14), (46, 16), (46, 34), (32, 48), (18, 34), (18, 16)], fill=(180, 190, 200))
    # Inner border
    draw.polygon([(32, 18), (42, 20), (42, 32), (32, 44), (22, 32), (22, 20)], fill=(30, 60, 120)) # Deep royal blue shield
    # Cross emblem inside
    draw.line([(32, 20), (32, 40)], fill=(255, 215, 0), width=2)
    draw.line([(24, 28), (40, 28)], fill=(255, 215, 0), width=2)

def draw_symbol_thermometer(draw, theme):
    """Draw a hardware thermometer and cooling fan."""
    # Thermometer frame
    draw.rectangle([20, 16, 24, 40], fill=(220, 220, 220)) # stem
    draw.ellipse([17, 36, 27, 46], fill=(220, 220, 220)) # bulb
    
    # Red liquid rising
    draw.ellipse([19, 38, 25, 44], fill=(255, 50, 50)) # red bulb
    draw.rectangle([21, 24, 23, 38], fill=(255, 50, 50)) # red stem rising
    
    # Hot fire sparkles on right
    draw.polygon([(38, 20), (34, 26), (36, 32), (40, 32), (42, 26)], fill=(255, 100, 0))

def draw_symbol_snow(draw, theme):
    """Draw a crystallization snowflake."""
    cx, cy = 32, 30
    # Star lines
    draw.line([(cx - 14, cy), (cx + 14, cy)], fill=(130, 220, 255), width=2)
    draw.line([(cx, cy - 14), (cx, cy + 14)], fill=(130, 220, 255), width=2)
    draw.line([(cx - 10, cy - 10), (cx + 10, cy + 10)], fill=(130, 220, 255), width=2)
    draw.line([(cx - 10, cy + 10), (cx + 10, cy - 10)], fill=(130, 220, 255), width=2)
    
    # Flakes spikes
    for angle in (0, 45, 90, 135, 180, 225, 270, 315):
        rad = math.radians(angle)
        sx = int(cx + 8 * math.cos(rad))
        sy = int(cy + 8 * math.sin(rad))
        draw.point((sx, sy), fill=(255, 255, 255))

def draw_symbol_coin(draw, theme):
    """Draw stack of gold coins or treasure vault."""
    # Chest block
    draw.rectangle([18, 26, 46, 46], fill=(139, 69, 19)) # brown wood chest
    draw.rectangle([20, 28, 44, 44], fill=(160, 82, 45))
    # Iron bands
    draw.line([(24, 26), (24, 46)], fill=(80, 80, 80), width=2)
    draw.line([(40, 26), (40, 46)], fill=(80, 80, 80), width=2)
    # Golden lock
    draw.rectangle([30, 34, 34, 38], fill=(255, 215, 0))
    draw.point((32, 37), fill=(0, 0, 0))
    
    # Gold coins spilling on top
    draw.ellipse([22, 18, 30, 25], fill=(255, 215, 0), outline=(184, 134, 11))
    draw.ellipse([32, 16, 42, 23], fill=(255, 215, 0), outline=(184, 134, 11))
    draw.ellipse([26, 20, 36, 27], fill=(255, 225, 50), outline=(184, 134, 11))

def draw_symbol_level_up(draw, theme):
    """Draw giant upward pointing arrow (level up)."""
    # Giant arrow shape pointing up
    # Coordinates of arrow: tail (26, 48) to (38, 48), neck at y=30, head pointing to (32, 14)
    draw.polygon([(32, 12), (18, 30), (26, 30), (26, 48), (38, 48), (38, 30), (46, 30)], fill=(0, 255, 100))
    
    # Golden highlights on arrow borders
    draw.polygon([(32, 16), (22, 28), (28, 28), (28, 46), (36, 46), (36, 28), (42, 28)], fill=(120, 255, 180))
    # Star sparkles in background
    draw.point((14, 20), fill=(255, 255, 255))
    draw.point((48, 38), fill=(255, 255, 255))

def draw_symbol_share(draw, theme):
    """Draw share nodes / export cards."""
    # Outward share node pattern
    # Central node at (24, 32), top right (40, 20), bottom right (40, 44)
    draw.line([(24, 32), (40, 20)], fill=(200, 200, 200), width=2)
    draw.line([(24, 32), (40, 44)], fill=(200, 200, 200), width=2)
    
    draw.ellipse([18, 26, 29, 37], fill=(30, 144, 255), outline=(240, 240, 240)) # Center blue node
    draw.ellipse([34, 14, 45, 25], fill=(255, 100, 100), outline=(240, 240, 240)) # Top red node
    draw.ellipse([34, 38, 45, 49], fill=(0, 200, 100), outline=(240, 240, 240)) # Bottom green node

def draw_symbol_hidden(draw, theme):
    """Draw crescent moon / key / secret question mark / glitch block."""
    # Secret key & Lock or Question mark
    # Draw question mark in red/gold glitchy style
    draw.arc([22, 16, 42, 34], 180, 360, fill=(255, 80, 80), width=2)
    draw.line([(42, 25), (32, 34)], fill=(255, 80, 80), width=2)
    draw.line([(32, 34), (32, 38)], fill=(255, 80, 80), width=2)
    draw.point((32, 42), fill=(255, 80, 80))
    
    # Shimmering sparkles around
    draw.point((16, 16), fill=(255, 255, 0))
    draw.point((48, 18), fill=(255, 255, 0))
    draw.point((18, 46), fill=(255, 255, 0))
    draw.point((46, 44), fill=(255, 255, 0))

def draw_symbol_lightbulb(draw, theme):
    """Draw a glowing yellow lightbulb (insight)."""
    # Bulb top circle
    draw.ellipse([22, 14, 42, 34], fill=(255, 235, 50), outline=(230, 160, 0))
    # Bulb glass base
    draw.rectangle([26, 30, 38, 36], fill=(255, 235, 50))
    # Metal base thread
    draw.rectangle([28, 37, 36, 42], fill=(170, 170, 170))
    draw.line([(29, 39), (35, 39)], fill=(120, 120, 120))
    draw.line([(29, 41), (35, 41)], fill=(120, 120, 120))
    # Black contact point
    draw.point((32, 43), fill=(40, 40, 40))
    
    # Glow rays
    draw.line([(18, 24), (14, 24)], fill=(255, 255, 0))
    draw.line([(46, 24), (50, 24)], fill=(255, 255, 0))
    draw.line([(32, 10), (32, 6)], fill=(255, 255, 0))

def draw_symbol_explore(draw, theme):
    """Draw magnifying glass / eye looking at map."""
    # Magnifying glass handle
    draw.line([(34, 32), (48, 46)], fill=(139, 90, 43), width=3)
    # Glass ring
    draw.ellipse([16, 14, 36, 34], fill=(100, 200, 255), outline=(200, 200, 200))
    # Glass highlight reflection
    draw.line([(20, 18), (28, 18)], fill=(255, 255, 255))
    draw.line([(20, 18), (20, 24)], fill=(255, 255, 255))

def draw_symbol_worklog_card(draw, theme):
    """Draw a stylized worklog rarity card with a diamond emblem."""
    # Card shadow (slightly offset)
    draw.rectangle([24, 17, 43, 45], fill=theme['shadow'])
    # Card body (cream/parchment)
    draw.rectangle([22, 15, 41, 43], fill=(238, 232, 220))
    # Colored header bar at top
    draw.rectangle([22, 15, 41, 21], fill=theme['main'])
    # Fold corner (top-right)
    draw.polygon([(37, 15), (41, 19), (37, 19)], fill=theme['shadow'])
    draw.point((41, 15), fill=(16, 16, 18))
    draw.point((40, 15), fill=(16, 16, 18))
    draw.point((41, 16), fill=(16, 16, 18))
    # Central diamond emblem
    draw.polygon([(32, 24), (37, 29), (32, 34), (27, 29)], fill=theme['main'])
    draw.polygon([(32, 26), (35, 29), (32, 32), (29, 29)], fill=theme['light'])
    draw.point((32, 29), fill=theme['shadow'])
    # Data lines at bottom of card
    draw.line([(26, 37), (38, 37)], fill=(180, 180, 170))
    draw.line([(26, 40), (34, 40)], fill=(180, 180, 170))

def draw_symbol_cards_fan(draw, theme):
    """Draw multiple cards fanned out (for A-tier rarity collection)."""
    # Back-left card
    draw.rectangle([13, 20, 28, 42], fill=(180, 175, 165))
    draw.rectangle([13, 20, 28, 25], fill=(100, 160, 200))
    draw.rectangle([13, 20, 28, 42], outline=theme['shadow'])
    draw.point((18, 31), fill=(80, 130, 170))
    # Back-right card
    draw.rectangle([36, 20, 51, 42], fill=(180, 175, 165))
    draw.rectangle([36, 20, 51, 25], fill=theme['shadow'])
    draw.rectangle([36, 20, 51, 42], outline=theme['shadow'])
    draw.point((44, 31), fill=theme['shadow'])
    # Front center card (on top)
    draw.rectangle([22, 14, 41, 44], fill=(238, 232, 220))
    draw.rectangle([22, 14, 41, 20], fill=theme['main'])
    draw.rectangle([22, 14, 41, 44], outline=theme['shadow'])
    # Diamond emblem on front card
    draw.polygon([(31, 24), (36, 29), (31, 34), (26, 29)], fill=theme['main'])
    draw.polygon([(31, 26), (34, 29), (31, 32), (28, 29)], fill=theme['light'])

def draw_symbol_cards_stack(draw, theme):
    """Draw a stacked pile of cards (for S-tier 30-card milestone)."""
    # Stacked depth layers
    for i in range(4, 0, -1):
        shade = max(140, 200 - i * 15)
        draw.rectangle(
            [18 + i, 14 + i * 2, 44 + i // 2, 42 + i // 2],
            fill=(shade, max(0, shade - 5), max(0, shade - 10))
        )
    # Top card
    draw.rectangle([18, 14, 44, 42], fill=(238, 232, 220))
    draw.rectangle([18, 14, 44, 20], fill=theme['main'])
    draw.rectangle([18, 14, 44, 42], outline=theme['shadow'])
    # Fold corner on top card
    draw.polygon([(40, 14), (44, 18), (40, 18)], fill=theme['shadow'])
    draw.point((44, 14), fill=(16, 16, 18))
    draw.point((43, 14), fill=(16, 16, 18))
    draw.point((44, 15), fill=(16, 16, 18))
    # Diamond on top card
    draw.polygon([(31, 24), (36, 29), (31, 34), (26, 29)], fill=theme['main'])
    draw.polygon([(31, 26), (34, 29), (31, 32), (28, 29)], fill=theme['light'])
    # Sparkle highlights suggesting treasure
    draw.point((15, 16), fill=theme['light'])
    draw.point((50, 18), fill=theme['light'])

def draw_symbol_pressure_gauge(draw, theme):
    """Draw a high-pressure steam gauge (for pressure repairer achievement)."""
    # Outer gauge ring
    draw.ellipse([16, 12, 48, 44], fill=(180, 180, 190), outline=theme['shadow'])
    # Inner gauge face
    draw.ellipse([19, 15, 45, 41], fill=(235, 230, 215))
    # Red danger-zone tick marks (upper-right arc)
    for angle_deg in range(-60, 30, 10):
        angle = math.radians(angle_deg)
        cx, cy = 32, 28
        x1 = int(cx + 10 * math.cos(angle))
        y1 = int(cy + 10 * math.sin(angle))
        x2 = int(cx + 12 * math.cos(angle))
        y2 = int(cy + 12 * math.sin(angle))
        draw.line([(x1, y1), (x2, y2)], fill=(255, 60, 60))
    # Normal tick marks
    for angle_deg in range(30, 250, 30):
        angle = math.radians(angle_deg)
        cx, cy = 32, 28
        x1 = int(cx + 10 * math.cos(angle))
        y1 = int(cy + 10 * math.sin(angle))
        x2 = int(cx + 12 * math.cos(angle))
        y2 = int(cy + 12 * math.sin(angle))
        draw.line([(x1, y1), (x2, y2)], fill=(120, 120, 120))
    # Needle pointing to danger zone (upper-right)
    cx, cy = 32, 28
    needle_angle = math.radians(-45)
    nx = int(cx + 9 * math.cos(needle_angle))
    ny = int(cy + 9 * math.sin(needle_angle))
    draw.line([(cx, cy), (nx, ny)], fill=(255, 50, 50))
    draw.point((cx, cy), fill=(60, 60, 60))
    draw.point((cx + 1, cy), fill=(60, 60, 60))
    # Steam pipes at bottom
    draw.rectangle([29, 43, 35, 48], fill=(150, 155, 165))
    draw.rectangle([22, 45, 28, 50], fill=(150, 155, 165))
    draw.rectangle([36, 45, 42, 50], fill=(150, 155, 165))
    # Steam wisps
    draw.point((20, 44), fill=(190, 200, 255))
    draw.point((19, 42), fill=(190, 200, 255))
    draw.point((44, 44), fill=(190, 200, 255))
    draw.point((45, 42), fill=(190, 200, 255))

def draw_symbol_rank_badge(draw, theme):
    """Draw a rank medal for job title level achievements."""
    # Ribbon at top (two diagonal bands)
    draw.polygon([(24, 14), (32, 14), (24, 22)], fill=(200, 50, 50))
    draw.polygon([(32, 14), (40, 14), (40, 22), (32, 22)], fill=(170, 30, 30))
    draw.line([(28, 14), (28, 22)], fill=(220, 70, 70))
    # Medal circle
    draw.ellipse([16, 22, 48, 50], fill=theme['main'], outline=theme['shadow'])
    draw.ellipse([18, 24, 46, 48], fill=theme['light'])
    # 5-pointed star in center
    star_cx, star_cy = 32, 36
    star_points = []
    for i in range(10):
        angle = math.radians(-90 + i * 36)
        r = 9 if i % 2 == 0 else 4
        star_points.append((
            int(star_cx + r * math.cos(angle)),
            int(star_cy + r * math.sin(angle))
        ))
    draw.polygon(star_points, fill=theme['main'])

def draw_symbol_crown_all(draw, theme):
    """Draw a royal crown with 7 gems (one per job title type)."""
    # Crown base band
    draw.rectangle([14, 38, 50, 48], fill=theme['main'])
    draw.rectangle([15, 39, 49, 47], fill=theme['light'])
    # Crown body with 5 alternating points
    draw.polygon([
        (14, 38), (14, 24), (20, 32), (26, 16),
        (32, 28), (38, 16), (44, 32), (50, 24), (50, 38)
    ], fill=theme['main'])
    # Crown body highlight
    draw.polygon([
        (15, 38), (15, 26), (20, 33), (26, 18),
        (32, 30), (38, 18), (44, 33), (49, 26), (49, 38)
    ], fill=theme['light'])
    # 7 gems representing 7 title families
    gem = (255, 110, 110)
    draw.rectangle([25, 14, 28, 17], fill=gem)   # left peak
    draw.rectangle([36, 14, 39, 17], fill=gem)   # right peak
    draw.rectangle([19, 29, 21, 32], fill=gem)   # left side
    draw.rectangle([43, 29, 45, 32], fill=gem)   # right side
    draw.rectangle([31, 24, 33, 27], fill=gem)   # center peak
    draw.rectangle([22, 40, 25, 43], fill=gem)   # band left
    draw.rectangle([39, 40, 42, 43], fill=gem)   # band right

def draw_badge_image(achievement):
    """Draw and save a badge image based on difficulty, category, and ID mapping."""
    # Create image
    img = Image.new("RGB", (64, 64), (16, 16, 18))
    draw = ImageDraw.Draw(img)
    
    difficulty = achievement['difficulty']
    theme = DIFFICULTY_THEMES[difficulty]
    
    # Draw frame
    draw_frame(draw, difficulty)
    
    # Retrieve specific symbol & label
    id_ = achievement['id']
    design_info = ACHIEVEMENT_DESIGNS.get(id_)
    if design_info:
        symbol, label = design_info
    else:
        # fallback based on category
        symbol, label = 'hidden', ''
        
    # Draw central symbol
    if symbol == 'cat_awake':
        draw_symbol_cat_awake(draw, theme)
    elif symbol == 'cat_talk':
        draw_symbol_cat_talk(draw, theme)
    elif symbol == 'cat_pet':
        draw_symbol_cat_pet(draw, theme)
    elif symbol == 'cat_drag':
        draw_symbol_cat_drag(draw, theme)
    elif symbol == 'clock':
        draw_symbol_clock(draw, theme)
    elif symbol == 'report':
        draw_symbol_report(draw, theme)
    elif symbol == 'terminal':
        draw_symbol_terminal(draw, theme)
    elif symbol == 'parts':
        draw_symbol_parts(draw, theme)
    elif symbol == 'settings':
        draw_symbol_settings(draw, theme)
    elif symbol == 'keyboard':
        draw_symbol_keyboard(draw, theme)
    elif symbol == 'mouse':
        draw_symbol_mouse(draw, theme)
    elif symbol == 'flame':
        draw_symbol_flame(draw, theme)
    elif symbol == 'calendar':
        draw_symbol_calendar(draw, theme)
    elif symbol == 'database':
        draw_symbol_database(draw, theme)
    elif symbol == 'network':
        draw_symbol_network(draw, theme)
    elif symbol == 'shield':
        draw_symbol_shield(draw, theme)
    elif symbol == 'thermometer':
        draw_symbol_thermometer(draw, theme)
    elif symbol == 'snow':
        draw_symbol_snow(draw, theme)
    elif symbol == 'coin':
        draw_symbol_coin(draw, theme)
    elif symbol == 'level_up':
        draw_symbol_level_up(draw, theme)
    elif symbol == 'share':
        draw_symbol_share(draw, theme)
    elif symbol == 'hidden':
        draw_symbol_hidden(draw, theme)
    elif symbol == 'lightbulb':
        draw_symbol_lightbulb(draw, theme)
    elif symbol == 'explore':
        draw_symbol_explore(draw, theme)
    elif symbol == 'worklog_card':
        draw_symbol_worklog_card(draw, theme)
    elif symbol == 'cards_fan':
        draw_symbol_cards_fan(draw, theme)
    elif symbol == 'cards_stack':
        draw_symbol_cards_stack(draw, theme)
    elif symbol == 'pressure_gauge':
        draw_symbol_pressure_gauge(draw, theme)
    elif symbol == 'rank_badge':
        draw_symbol_rank_badge(draw, theme)
    elif symbol == 'crown_all':
        draw_symbol_crown_all(draw, theme)
        
    # Draw label text if any
    if label:
        # Calculate width of label to center/right-align it
        total_width = 0
        for char in label:
            total_width += 5 if char in ('M', 'm') else 3
            total_width += 1
        total_width -= 1
        
        # position label at bottom right inside the shield: x_start = 45 - total_width, y_start = 42
        x_pos = int(48 - total_width)
        y_pos = 42
        
        # Draw a tiny black drop shadow/border behind text
        draw_text_pixel(draw, label, x_pos + 1, y_pos, (0, 0, 0))
        draw_text_pixel(draw, label, x_pos - 1, y_pos, (0, 0, 0))
        draw_text_pixel(draw, label, x_pos, y_pos + 1, (0, 0, 0))
        draw_text_pixel(draw, label, x_pos, y_pos - 1, (0, 0, 0))
        
        # Draw colored text
        draw_text_pixel(draw, label, x_pos, y_pos, theme['text'])
        
    # Scale up using NEAREST filter to preserve crisp pixel edges
    scaled_img = img.resize((256, 256), Image.Resampling.NEAREST)
    
    # Save as WebP
    badge_key = achievement['badge_key']
    file_path = os.path.join(OUTPUT_DIR, f"{badge_key}.webp")
    scaled_img.save(file_path, "WEBP")
    print(f"Generated {achievement['id']}: {badge_key}.webp")

def generate_placeholder():
    """Generate default placeholder badge image."""
    img = Image.new("RGB", (64, 64), (16, 16, 18))
    draw = ImageDraw.Draw(img)
    
    # Draw default entry frame/shield
    draw_frame(draw, 'entry')
    # Draw dark gray question mark in center
    draw_symbol_hidden(draw, DIFFICULTY_THEMES['entry'])
    
    scaled_img = img.resize((256, 256), Image.Resampling.NEAREST)
    file_path = os.path.join(OUTPUT_DIR, "cwp_badge_placeholder.webp")
    scaled_img.save(file_path, "WEBP")
    print("Generated default placeholder: cwp_badge_placeholder.webp")

def main():
    print("Parsing achievements from spec document...")
    achievements = parse_achievements()
    print(f"Found {len(achievements)} achievements.")
    
    if len(achievements) == 0:
        print("Error: No achievements found in spec. Please check spec file path.")
        return
        
    print("Generating badges...")
    for achievement in achievements:
        draw_badge_image(achievement)
        
    generate_placeholder()
    print("All achievement badge images successfully generated!")

if __name__ == "__main__":
    main()
