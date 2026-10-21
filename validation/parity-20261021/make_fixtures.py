"""Original neutral PNG fixtures; standard library only. No repository code runs."""
import hashlib
import json
import pathlib
import struct
import sys
import zlib

out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(__file__).resolve().parent / 'fixtures'
out.mkdir(parents=True, exist_ok=True)
def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff)
def png(name, width, height, pixel):
    raw = b''.join(b'\x00' + b''.join(bytes(pixel(x,y)) for x in range(width)) for y in range(height))
    data = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB',width,height,8,6,0,0,0))
    data += chunk(b'pHYs', struct.pack('>IIB',11811,11811,1))
    data += chunk(b'IDAT', zlib.compress(raw,9)) + chunk(b'IEND',b'')
    (out/name).write_bytes(data)
def grid(x,y):
    if x<4 or y<4 or x>=508 or y>=508: return (255,0,255,255)
    if abs(x-256)<2 or abs(y-256)<2: return (255,255,255,255)
    if x%32<2 or y%32<2: return (0,0,0,255)
    return (x*255//511,y*255//511,128,255)
png('grid-512.png',512,512,grid)
png('opaque-grid-512.png',512,512,grid)
png('red-200.png',200,200,lambda x,y:(255,0,0,255))
png('blue-400x200.png',400,200,lambda x,y:(0,0,255,255))
png('blue-400x200-control.png',400,200,lambda x,y:(0,160,255,255))
png('soft-edge-control.png',512,512,lambda x,y:(32,96,224,int(255*min(1,max(0,min(x,y,511-x,511-y)/32)))))
png('pencil-64-transparent.png',64,64,lambda x,y:(0,0,0,0))
png('marquee-400x300.png',400,300,lambda x,y:(224 if (x//10+y//10)%2 else 192,)*3+(255,))
manifest = [{'file':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size} for p in sorted(out.glob('*.png'))]
(out/'fixture-manifest.json').write_text(json.dumps({'generator':'make_fixtures.py','provenance':'Original procedural neutral test images, MIT; 8-bit RGBA, 300 ppi; complex PSD/PSB/ATN inputs must be authored in Photoshop or sourced separately.','files':manifest},indent=2))
print(json.dumps({'directory':str(out),'fixtures':len(manifest)}))

