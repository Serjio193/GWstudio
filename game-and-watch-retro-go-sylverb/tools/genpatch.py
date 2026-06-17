import argparse
import struct
from pathlib import Path

def writestring(file, ss):
    "Write string data to file"
    file.write(bytes(ss, encoding="UTF-8",errors = "ignore"))

def writediff(src, dst, start, count, out):
    steps = (count + 15) // 16
    outs = [];
    counts = [];
    for i in range(steps):
        outs.append("")
        counts.append(0)
    for i in range(count):
        step = i // 16
        outs[step] = outs[step] + "%02x"%dst[start+i]
        counts[step] += 1
    for i in range(steps):
        #get count and address
        address = start + i * 16
        #counts[i] * 16 * 0x10000 + start
        writestring(out,"%06x"%((counts[i] - 1) * 16 * 0x10000 + address) + outs[i] +",")

def CompareOneFile(src_file, dst_file):
    src = open(src_file, "rb")
    src_data = src.read()
    src.close()
    dst = open(dst_file, "rb")
    dst_data = dst.read()
    dst.close()
    out_file = dst_file.with_suffix(".pceplus")
    out = open(out_file, "wb")
    isDiff = False
    diffCount = 0
    srcCount = len(src_data)
    dstCount = len(dst_data)
    for i in range(srcCount):
        if (i < dstCount):
            if (src_data[i] == dst_data[i]):
                if isDiff:
                    writediff(src_data, dst_data, i - diffCount, diffCount, out)
                    isDiff = False
                    diffCount = 0
            else:
                isDiff = True
                diffCount += 1
                if ((i == (dstCount - 1)) or (i == (srcCount - 1))): 
                    #the last is diff
                    writediff(src_data, dst_data, i - diffCount + 1, diffCount, out)
                    isDiff = False
                    diffCount = 0
    writestring(out,"\tHacked Version\n")
    

def ProcessFiles(src, dst, ext):
    #toDo
    src_files = list(Path(src).iterdir())
    src_files = [r for r in src_files if r.name.lower().endswith(ext)]
    src_files.sort()
    for src_file in src_files :
        #src_name = src_file.stem + ext
        #print(src_file)
        dst_file = Path(dst) / src_file.name
        #print(Path(dst) / src_file.name)
        if dst_file.exists():
            print("Process File: " + str(src_file))
            CompareOneFile(src_file, dst_file)

def main():
    import sys
    #filepath.stem
    if (len(sys.argv) != 3):
        print("Usage: " + sys.argv[0] + " OgnDir TagetDir")
        return
    ProcessFiles(sys.argv[1], sys.argv[2], ".pce")

if __name__ == "__main__":
    main()
