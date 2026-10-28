"""从成功C源码提取meta打包函数，独立编译并生成合成测试fixture（无固件）。"""
import argparse
import hashlib
import pathlib
import subprocess

p=argparse.ArgumentParser(description=__doc__)
p.add_argument("--source",type=pathlib.Path,required=True)
p.add_argument("--clang",type=pathlib.Path,required=True)
a=p.parse_args()
root=pathlib.Path(__file__).resolve().parents[1]
work=root/"local-artifacts/c-meta-witness"
work.mkdir(parents=True,exist_ok=True)
source=a.source.read_text(encoding="utf-8-sig")
start=source.index("typedef struct {\n    UINT64 magic;")
end=source.index("} GspFwWprMeta;",start)+len("} GspFwWprMeta;")
structure=source[start:end]
start=source.index("static void\nbuild_wpr_meta(")
end=source.index("\n}\n",start)+3
function=source[start:end]
stub="""#include <stdint.h>
#include <stdio.h>
#include <string.h>
typedef uint64_t UINT64; typedef uint32_t UINT32;
typedef uint16_t UINT16; typedef uint8_t UINT8;
#define SetMem(p,n,v) memset(p,v,n)
#define GSP_FW_WPR_META_MAGIC 0xdc3aae21371a60b3ULL
#define GSP_FW_WPR_META_REVISION 1
#define V67_SIZE 0xfa00
static int cmp90_stockSig=0;
"""
main="""
int main(void) {
 GspFwWprMeta m;
 if(sizeof(m)!=256) return 2;
 build_wpr_meta(&m,0x115c20000ULL,0x1bfadc8ULL,0x110000000ULL,
                0x580000000ULL,0x11201f000ULL,0x6000ULL);
 return fwrite(&m,1,sizeof(m),stdout)!=sizeof(m);
}
"""
program=work/"meta.c"
program.write_text(stub+structure+"\n"+function+main,encoding="utf-8")
exe=work/"meta.exe"
subprocess.run([str(a.clang),"-O2",str(program),"-o",str(exe)],check=True)
result=subprocess.check_output([str(exe)])
if len(result)!=256: raise ValueError("C meta output not 256 bytes")
fixture=root/"sim/tests/fixtures/tu102-c-meta.hex"
fixture.parent.mkdir(parents=True,exist_ok=True)
fixture.write_text(result.hex()+"\n",encoding="ascii")
print("C meta fixture SHA256="+hashlib.sha256(result).hexdigest())
