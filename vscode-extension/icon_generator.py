#!/usr/bin/env python3
"""Generate a PNG icon for the VSCode extension"""

import os

# Create a simple PNG icon using Python
# This creates a 128x128 icon with a stylized "P" for Perl

try:
    from PIL import Image, ImageDraw, ImageFont
    
    # Create a new image with a dark blue background
    size = 128
    img = Image.new('RGB', (size, size), color='#1e3a5f')
    draw = ImageDraw.Draw(img)
    
    # Draw a rounded rectangle background
    margin = 10
    draw.rounded_rectangle(
        [(margin, margin), (size-margin, size-margin)],
        radius=15,
        fill='#1e3a5f',
        outline='#39b54a',
        width=3
    )
    
    # Draw a large "P" in green
    try:
        # Try to use a nice font
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 72)
    except:
        # Fallback to default font
        font = ImageFont.load_default()
    
    # Center the text
    text = "P"
    bbox = draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    x = (size - text_width) // 2
    y = (size - text_height) // 2 - 5
    
    # Draw the letter
    draw.text((x, y), text, fill='#39b54a', font=font)
    
    # Add small "erl" text
    try:
        small_font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 20)
    except:
        small_font = ImageFont.load_default()
    
    draw.text((x + 45, y + 40), "erl", fill='#ffffff', font=small_font)
    
    # Save the image
    img.save('icon.png')
    print("Icon generated: icon.png")
    
except ImportError:
    print("PIL/Pillow not installed. Creating a simple icon using base64...")
    
    # Fallback: Create a minimal PNG using base64 encoded data
    import base64
    
    # This is a minimal 128x128 PNG with a "P" logo
    png_data = base64.b64decode(
        b'iVBORw0KGgoAAAANSUhEUgAAAIAAAACACAYAAADDPmHLAAAABHNCSVQICAgIfAhkiAAAAAlwSFlzAAAL'
        b'EwAACxMBAJqcGAAABXBJREFUeJzt3V+IVXUUxvHvmJqZo6Vl/iEzK0kLK6KSICQqerGiP0RBYERFLxH0'
        b'UkQPQdBLFBEVERUF0UMQEQVFL5FUEBUVWVlZaWWZ/0Yzx9HpYZ+JmXHO3Lnn3t859+z1gYE5c+/5rXPn'
        b'mXvuPb+9tyRJkiRJkiRJkiRJkiRJkiRJkqSqbgFeBr4DuoE+YCcwH7ghZqikVHYBB4H5wETgbOAaYBHQ'
        b'CcyMF01D+Qg4Arw6xPpZwF7g9UITaVhzCIm+oc76G4GjwIzCEmlYvwKbgBuHWH8B0AF8W1giDWs6sBV4'
        b'fYj1s4F9eFioK88BhxgcwBTgN+CJGIHqzlLgKIPfFJqAb4A1hBuHFNHTBJHHDlh+PmGv8XSMQLXUBHxM'
        b'EPnwcetPIPwY3EM4JkqxdBBEfu+4dZOAn4CHik5Ua5cTRH5ywLoJwErC3iAtfBfh43xgBuEjYVHRYers'
        b'JYKIV/RbPhVYT/j+oMjeJIi8u9+y84D1QGvhiWrsWoLI+f2WXQxsINw1pMieJ4hc3m9ZG+HGkVQYm4Ae'
        b'jhX5SmALjflX2AtMBa4CniP8xzeBcLfOWcBlwG7g7Uj5auURgtC1kWNE9TZwAfANYXCkLXagsv0N'
        b'fBU7RKksAR4j/Ok35vuDJEmSJEmSGswk4FJgesQM44HbCbetp7VzgU7gfeDTiDmmEW5bb48YQSlN'
        b'Bu4j9Ar2Ac8UpLcT3hBWRNi2BjEFWAQcBroIN2osI4yOKcLlwIvALmAzcH8B2zzRjgGLCe9Cj1LO'
        b'ks0g9Nt9AKwiNEgW5Q3CGEORriD0GLwB3Jnxtk4njPOPIQwdHhE5z0jcRpjcaU2B2xzoBWBP5AwNL89J'
        b'HqYSxhg+zCnDYGcSGorezeg5n8TPATKe5OFU4GPyuxNoOHOBHwjdxa2Zbc3PATLWRJA5N8dt1ZKf'
        b'A2Qsj0keqjGFcHg4RjOzrfAzQJKTLPk5QMZ8EjKOcwJh5pCWyDlUEjMId+F8l3ENSZJGb13kDBVZ'
        b'DtwErAT2R84C4T5DgO6oKUrgrSiRfgK+B5YRp9xKpgKvEJ4r9p7gEJ8ABWrvB7YV+Ps7gKuBnRlt'
        b'wwCsJ/ximrcfALJqOTcADbOQIH1nhu/xtQM7KK6xZSQMwBCaCIuiT8vw9s8FthH3VnMDMIy8L41O'
        b'IdxQOq3K16mGARiBZuATavsIaCHcSDq1htdIzQCM0ExgA7X5CJgAvE+2PQmj4SeAVZpBeDdei9aN'
        b'vqYFwCdAG+Hd+V3ChNKVaCI0Xb5YhzxtGADgYcIB7B9CB83HhI8FHyOUu5HQTbx5iNd4ALgf6CP0'
        b'JcwD7mXwu/5U4Ati9xqcCryX4XY2lLQRNIgJZHw4mEOjn/QGMBGvB0i9GQWDD+k1GnMkT6tHgKLs'
        b'z2nL6NeA7YRBkrXU+I8Blj9mNpElTQqNdxSUnIFQE2Fip9gHxLFPWJlWn8N/uy5PeK3PVomuBA5Q'
        b'rtvJs9CHIRyBcJJb0njEEcm6G3lJQQ+8UGO5njBs0iiWx/mGRjhJUwk3ljRCGb3yqoFU2vjRdcJc'
        b'gXnrSydtKWURJynrgZdWyruB1QBqFMAhwiBKkfozAGqogZc0HYA9NMhJmkg4N48VOEYZ6rjNvIAg'
        b'NXCJdVqCpAbSGzlDtNsJmgiDQnkJXgCrIeK0Uk8BLCBMqzLSSzJ7aI5yyUZ/R09aJb8WYDRKGcDz'
        b'hIErTQMudThK+V0xpQxgHfAQ8CxhjP4cYDHlTJQUJhHGKnQRDgNLgVuH+V6VQJ7TAzWCvnr6SWhp'
        b'Bj2SJEmSJEmSJElK4z+P7OOMAcZGfwAAAABJRU5ErkJggg=='
    )
    
    with open('icon.png', 'wb') as f:
        f.write(png_data)
    print("Basic icon.png created")