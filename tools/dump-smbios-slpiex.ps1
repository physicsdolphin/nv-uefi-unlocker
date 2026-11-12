# G614JV raw dump: SLPIEX core records + SMBIOS Type 17 records.
# Read-only, no admin needed. Paste the whole output back.
$code = @'
using System;
using System.Runtime.InteropServices;
public static class RawDump {
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool GetLogicalProcessorInformationEx(uint rel, IntPtr buf, ref uint len);
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern uint GetSystemFirmwareTable(uint provider, uint table, IntPtr buf, uint len);
}
'@
Add-Type -TypeDefinition $code

# --- SLPIEX, RelationProcessorCore(0) ---
$len = [uint32]0
[void][RawDump]::GetLogicalProcessorInformationEx(0, [IntPtr]::Zero, [ref]$len)
$buf = [Runtime.InteropServices.Marshal]::AllocHGlobal([int64]$len)
[void][RawDump]::GetLogicalProcessorInformationEx(0, $buf, [ref]$len)
$slpiex = New-Object byte[] $len
[Runtime.InteropServices.Marshal]::Copy($buf, $slpiex, 0, $len)
[Runtime.InteropServices.Marshal]::FreeHGlobal($buf)
"SLPIEX_CORE len=$len"
for ($i = 0; $i -lt $len; $i += 16) {
  $row = $slpiex[$i..([Math]::Min($i+15, $len-1))]
  "  {0:x4}: {1}" -f $i, (($row | % { $_.ToString('x2') }) -join ' ')
}

# --- SMBIOS Type 17 ---
$tlen = [RawDump]::GetSystemFirmwareTable(0x52534D42, 0, [IntPtr]::Zero, 0)
$tbuf = [Runtime.InteropServices.Marshal]::AllocHGlobal([int64]$tlen)
[void][RawDump]::GetSystemFirmwareTable(0x52534D42, 0, $tbuf, $tlen)
$t = New-Object byte[] $tlen
[Runtime.InteropServices.Marshal]::Copy($tbuf, $t, 0, $tlen)
[Runtime.InteropServices.Marshal]::FreeHGlobal($tbuf)
$data = $t[8..($tlen-1)]
$off = 0
while ($off + 4 -le $data.Length) {
  $typ = $data[$off]; $ln = $data[$off+1]
  if ($ln -lt 4 -or $off + $ln -gt $data.Length) { break }
  if ($typ -eq 17) {
    $h = [BitConverter]::ToUInt16($data, $off+2)
    "=== TYPE17 len=0x{0:x2} handle=0x{1:x4} ===" -f $ln, $h
    $row = $data[$off..($off+$ln-1)]
    "  raw: " + (($row | % { $_.ToString('x2') }) -join ' ')
    $p = $off + $ln; $idx = 1
    while ($p -lt $data.Length -and $data[$p] -ne 0) {
      $q = $p
      while ($q -lt $data.Length -and $data[$q] -ne 0) { $q++ }
      "  str$idx = " + [Text.Encoding]::ASCII.GetString($data, $p, $q-$p)
      $idx++; $p = $q + 1
    }
  }
  if ($typ -eq 127) { break }
  $off += $ln
  $nn = 0
  while ($off -lt $data.Length -and $nn -lt 2) {
    if ($data[$off] -eq 0) { $nn++ } else { $nn = 0 }
    $off++
  }
}
"DUMP_END"
