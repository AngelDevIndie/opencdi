with open('C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/striker_frame_3501.bmp', 'rb') as f1:
    b1 = f1.read()
with open('C:/Users/Gian/.gemini/antigravity/brain/47421117-6e2a-4536-8928-519a0c9b76ed/striker_frame_3502.bmp', 'rb') as f2:
    b2 = f2.read()

diffs = 0
diff_list = []
for i in range(54, len(b1), 4):
    if b1[i:i+3] != b2[i:i+3]:
        diffs += 1
        px = (i - 54) // 4
        y = px // 768
        x = px % 768
        if len(diff_list) < 20:
            c1 = (b1[i+2], b1[i+1], b1[i])
            c2 = (b2[i+2], b2[i+1], b2[i])
            diff_list.append((x, y, c1, c2))

print(f"Diff 3501 vs 3502: {diffs} pixels")
for d in diff_list:
    print(d)
