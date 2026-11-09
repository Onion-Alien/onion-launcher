<# Runs a POC self-test on a hidden Windows desktop ("ql_test"): its windows
   exist and render, but never reach the real screen or take focus. #>
param([Parameter(Mandatory)][string]$CommandLine, [int]$TimeoutSeconds = 40)
$ErrorActionPreference = "Stop"
if (-not ("QlHiddenDesk" -as [type])) {
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class QlHiddenDesk {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct StartupInfo {
        public int cb; public string reserved; public string desktop; public string title;
        public int x, y, xSize, ySize, xChars, yChars, fill, flags;
        public short show, reserved2; public IntPtr reserved3, stdIn, stdOut, stdErr;
    }
    [StructLayout(LayoutKind.Sequential)]
    struct ProcessInfo { public IntPtr process, thread; public int pid, tid; }
    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern IntPtr CreateDesktop(string name, IntPtr device, IntPtr mode, int flags, uint access, IntPtr sa);
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern bool CreateProcess(string app, string cmd, IntPtr pa, IntPtr ta, bool inherit, uint flags,
        IntPtr env, string dir, ref StartupInfo si, out ProcessInfo pi);
    [DllImport("kernel32.dll")] static extern uint WaitForSingleObject(IntPtr h, uint ms);
    [DllImport("kernel32.dll")] static extern bool GetExitCodeProcess(IntPtr h, out uint code);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
    public static int LastPid;
    public static int Run(string cmd, int timeoutMs) {
        IntPtr desk = CreateDesktop("ql_test", IntPtr.Zero, IntPtr.Zero, 0, 0x10000000, IntPtr.Zero);
        if (desk == IntPtr.Zero) throw new Exception("CreateDesktop failed: " + Marshal.GetLastWin32Error());
        StartupInfo si = new StartupInfo(); si.cb = Marshal.SizeOf(si);
        si.desktop = "WinSta0" + (char)92 + "ql_test";
        ProcessInfo pi;
        if (!CreateProcess(null, cmd, IntPtr.Zero, IntPtr.Zero, false, 0x08000000, IntPtr.Zero, null, ref si, out pi))
            throw new Exception("CreateProcess failed: " + Marshal.GetLastWin32Error());
        LastPid = pi.pid;
        uint wait = WaitForSingleObject(pi.process, (uint)timeoutMs);
        uint code = 0; GetExitCodeProcess(pi.process, out code);
        CloseHandle(pi.thread); CloseHandle(pi.process);
        return wait == 0 ? (int)code : -1;
    }
}
"@
}
$code = [QlHiddenDesk]::Run($CommandLine, $TimeoutSeconds * 1000)
if ($code -eq -1) {
    Stop-Process -Id ([QlHiddenDesk]::LastPid) -Force -ErrorAction SilentlyContinue
    "TIMEOUT (killed pid $([QlHiddenDesk]::LastPid))"
} else { "exit=$code" }
