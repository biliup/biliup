# Read-only DXGI/WDDM probe. All device/counter joins use the adapter LUID.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)

$source = @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class BiliupGpuDxgi
{
    [StructLayout(LayoutKind.Sequential)]
    private struct Luid { public uint LowPart; public int HighPart; }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct AdapterDesc1
    {
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)]
        public string Description;
        public uint VendorId, DeviceId, SubSysId, Revision;
        public UIntPtr DedicatedVideoMemory, DedicatedSystemMemory, SharedSystemMemory;
        public Luid AdapterLuid;
        public uint Flags;
    }

    public sealed class AdapterInfo
    {
        public string Name;
        public uint VendorId;
        public string Luid;
        public ulong DedicatedVideoMemory;
    }

    [DllImport("dxgi.dll", ExactSpelling = true, CallingConvention = CallingConvention.StdCall)]
    private static extern int CreateDXGIFactory1(ref Guid iid, out IntPtr factory);

    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate int EnumAdapters1(IntPtr self, uint index, out IntPtr adapter);

    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate int GetDesc1(IntPtr self, out AdapterDesc1 desc);

    private static Delegate Method(IntPtr instance, int slot, Type delegateType)
    {
        IntPtr vtable = Marshal.ReadIntPtr(instance);
        return Marshal.GetDelegateForFunctionPointer(
            Marshal.ReadIntPtr(vtable, checked(slot * IntPtr.Size)), delegateType);
    }

    public static AdapterInfo[] Adapters()
    {
        // IDXGIFactory1 GUID. Vtable slots include IUnknown and IDXGIObject:
        // factory EnumAdapters1 = 12; IDXGIAdapter1 GetDesc1 = 10.
        Guid iid = new Guid("770aae78-f26f-4dba-a829-253c83d1b387");
        IntPtr factory = IntPtr.Zero;
        List<AdapterInfo> adapters = new List<AdapterInfo>();
        try
        {
            Marshal.ThrowExceptionForHR(CreateDXGIFactory1(ref iid, out factory));
            if (factory == IntPtr.Zero) throw new InvalidOperationException("DXGI returned no factory");
            EnumAdapters1 enumerate = (EnumAdapters1)Method(factory, 12, typeof(EnumAdapters1));
            for (uint index = 0; index < 64; index++)
            {
                IntPtr adapter = IntPtr.Zero;
                try
                {
                    int hr = enumerate(factory, index, out adapter);
                    if (hr == unchecked((int)0x887A0002)) break; // DXGI_ERROR_NOT_FOUND
                    Marshal.ThrowExceptionForHR(hr);
                    if (adapter == IntPtr.Zero) throw new InvalidOperationException("DXGI returned no adapter");
                    AdapterDesc1 desc;
                    GetDesc1 describe = (GetDesc1)Method(adapter, 10, typeof(GetDesc1));
                    Marshal.ThrowExceptionForHR(describe(adapter, out desc));
                    // DXGI software adapters have no physical WDDM engine or
                    // dedicated-memory telemetry. Keep hardware NVIDIA,
                    // Intel, and AMD adapters; the caller can fall back to
                    // nvidia-smi only if this complete WDDM probe fails.
                    if ((desc.Flags & 2) != 0 || (desc.VendorId != 0x8086 && desc.VendorId != 0x1002 && desc.VendorId != 0x10de)) continue;
                    adapters.Add(new AdapterInfo {
                        Name = desc.Description,
                        VendorId = desc.VendorId,
                        Luid = unchecked((uint)desc.AdapterLuid.HighPart).ToString("x8") + ":" + desc.AdapterLuid.LowPart.ToString("x8"),
                        DedicatedVideoMemory = desc.DedicatedVideoMemory.ToUInt64()
                    });
                }
                finally { if (adapter != IntPtr.Zero) Marshal.Release(adapter); }
            }
        }
        finally { if (factory != IntPtr.Zero) Marshal.Release(factory); }
        return adapters.ToArray();
    }
}
'@

$result = [ordered]@{
    Adapters = @()
    Engines = @()
    Memory = @()
    InventoryError = $null
    EngineError = $null
    MemoryError = $null
}
try {
    Add-Type -TypeDefinition $source -Language CSharp
    $result.Adapters = @([BiliupGpuDxgi]::Adapters())
} catch {
    $result.InventoryError = $_.Exception.Message
    ConvertTo-Json -InputObject $result -Depth 5 -Compress
    exit 0
}

$adapterLuids = @{}
foreach ($adapter in $result.Adapters) { $adapterLuids[$adapter.Luid] = $true }

try {
    # Aggregate process samples before JSON serialization to keep the bounded
    # output small even when many processes use the same GPU engine.
    $engines = @{}
    foreach ($sample in @(Get-CimInstance -ClassName Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine -Property Name,UtilizationPercentage -OperationTimeoutSec 2)) {
        if ($sample.Name -match '^(?:pid_[0-9]+_)?(luid_0x([0-9a-fA-F]{1,8})_0x([0-9a-fA-F]{1,8})_phys_[0-9]+_eng_[0-9]+_engtype_.+)$') {
            $luid = $Matches[2].PadLeft(8, '0').ToLowerInvariant() + ':' + $Matches[3].PadLeft(8, '0').ToLowerInvariant()
            if (-not $adapterLuids.ContainsKey($luid) -or $null -eq $sample.UtilizationPercentage) { continue }
            $key = $Matches[1].ToLowerInvariant()
            if (-not $engines.ContainsKey($key)) {
                $engines[$key] = [ordered]@{ Name = $Matches[1]; UtilizationPercentage = [double]0 }
            }
            $engines[$key].UtilizationPercentage += [double]$sample.UtilizationPercentage
        }
    }
    $result.Engines = @($engines.Values)
} catch { $result.EngineError = $_.Exception.Message }

try {
    $result.Memory = @(Get-CimInstance -ClassName Win32_PerfFormattedData_GPUPerformanceCounters_GPUAdapterMemory -Property Name,DedicatedUsage -OperationTimeoutSec 2 | ForEach-Object {
        if ($_.Name -match '^luid_0x([0-9a-fA-F]{1,8})_0x([0-9a-fA-F]{1,8})_phys_[0-9]+$') {
            $luid = $Matches[1].PadLeft(8, '0').ToLowerInvariant() + ':' + $Matches[2].PadLeft(8, '0').ToLowerInvariant()
            if ($adapterLuids.ContainsKey($luid)) {
                [ordered]@{ Name = $_.Name; DedicatedUsage = $_.DedicatedUsage }
            }
        }
    })
} catch { $result.MemoryError = $_.Exception.Message }

ConvertTo-Json -InputObject $result -Depth 5 -Compress
