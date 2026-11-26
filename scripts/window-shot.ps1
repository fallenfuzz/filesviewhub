# Saves a picture of the first visible window whose title matches $match (a -like pattern), for
# checks by eye of the Linux dev app under WSLg, which has no DevTools. Run on Windows:
# `powershell -File scripts\window-shot.ps1 "*(Ubuntu-24.04)" out.png`. PrintWindow draws the
# window even when another one covers it.
param([string]$match, [string]$out)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Text;
public static class W {
  public delegate bool P(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(P f, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  public struct RECT { public int L, T, R, B; }
}
"@
$found = $null
[W]::EnumWindows({ param($h, $l)
  $sb = New-Object System.Text.StringBuilder 512; [void][W]::GetWindowText($h, $sb, 512)
  if (-not $script:found -and [W]::IsWindowVisible($h) -and $sb.ToString() -like $match) { $script:found = $h; $script:title = $sb.ToString() }
  $true }, [IntPtr]::Zero) | Out-Null
if (-not $found) { "no window like $match"; exit 1 }
$r = New-Object W+RECT; [void][W]::GetWindowRect($found, [ref]$r)
$bmp = New-Object System.Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T)
$g = [System.Drawing.Graphics]::FromImage($bmp); $dc = $g.GetHdc()
[void][W]::PrintWindow($found, $dc, 2); $g.ReleaseHdc($dc)
$bmp.Save($out); "$title $($bmp.Width)x$($bmp.Height)"
