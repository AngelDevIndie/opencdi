with open('C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/mario_flicker_4000.bmp', 'rb') as f:
    bmp = f.read()

for y in range(430, 442):
    row = []
    for x in range(200, 222):
        idx = 54 + (y * 768 + x) * 4
        b, g, r = bmp[idx], bmp[idx+1], bmp[idx+2]
        row.append(f"({r:02X},{g:02X},{b:02X})")
    print(f"y={y}: " + " ".join(row))
