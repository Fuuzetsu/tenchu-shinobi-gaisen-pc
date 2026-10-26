using System;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using BizHawk.Emulation.Cores.Waterbox;

namespace OnikageSimCapture
{
    // The archived CadenceProbe verifies the exact core and CPU/DCache mapping.
    // This hook only observes executed instructions; it never changes game RAM.
    internal sealed class PhaseProbe : IDisposable
    {
        [DllImport("libtenchu_retail_probe.so")] static extern IntPtr retail_probe_begin(IntPtr cpu, IntPtr ram);
        [DllImport("libtenchu_retail_probe.so")] static extern int retail_probe_field(uint field, long cycles);
        [DllImport("libtenchu_retail_probe.so")] static extern int retail_probe_size();
        [DllImport("libtenchu_retail_probe.so")] static extern ulong retail_probe_error();
        [DllImport("libtenchu_retail_probe.so")] static extern int retail_probe_take([Out] byte[] bytes, UIntPtr size);
        [DllImport("libtenchu_retail_probe.so")] static extern void retail_probe_end();
        readonly WaterboxMemoryDomainPointer ram;
        readonly IntPtr cpu, hook;
        readonly GZipStream stream;
        byte[] buffer = new byte[0];
        bool installed, disposed;
        int count;

        internal PhaseProbe(WaterboxCore emulator, WaterboxMemoryDomainPointer memory,
                            IntPtr cpuPointer, int fields, uint[] ranges)
        {
            if (Environment.OSVersion.Platform != PlatformID.Unix || fields > 1000000)
                throw new Exception("Retail observer requires the pinned Linux runtime and a bounded tape");
            ram = memory; cpu = cpuPointer;
            var host = (WaterboxHost)typeof(WaterboxCore).GetField("_exe", BindingFlags.Instance | BindingFlags.NonPublic).GetValue(emulator);
            IntPtr callback = retail_probe_begin(cpu, ram.Definition.Data);
            if (callback == IntPtr.Zero) throw new Exception("Retail observer already active");
            try
            {
                hook = host.GetCallbackProcAddr(callback, 63);
                var path = Path.Combine(Environment.GetEnvironmentVariable("TENCHU_SETUP_OUTPUT"), "updates.ram.gz");
                stream = new GZipStream(new FileStream(path, FileMode.CreateNew), CompressionLevel.Fastest);
                var magic = Encoding.ASCII.GetBytes("TNRUN002"); stream.Write(magic, 0, magic.Length);
            }
            catch { retail_probe_end(); throw; }
        }

        void Flush()
        {
            int size = retail_probe_size();
            if (size < 0) throw new Exception("Retail observer overflow or unsupported CPU state: " + retail_probe_error().ToString("x16"));
            if (size == 0) return;
            if (buffer.Length < size) buffer = new byte[size];
            if (retail_probe_take(buffer, (UIntPtr)size) != 0) throw new Exception("Retail observer copy failed");
            stream.Write(buffer, 0, size);
        }

        internal void BeforeFrame(int field, long cycles)
        {
            if (disposed) throw new Exception("Disposed observer");
            Flush();
            if (field != ++count || retail_probe_field((uint)field, cycles) != 0)
                throw new Exception("Retail observer field discontinuity");
            if (installed) return;
            ram.Enter();
            try
            {
                if (Marshal.ReadIntPtr(cpu, 0x92598) != IntPtr.Zero || Marshal.ReadIntPtr(cpu, 0x925a0) != IntPtr.Zero)
                    throw new Exception("Another CPU debugger hook is installed");
                Marshal.WriteIntPtr(cpu, 0x92598, hook); installed = true;
            }
            finally { ram.Exit(); }
        }

        internal void Finish(string trace, byte[] traceHash, byte[] coreHash)
        {
            Flush();
            File.WriteAllText(trace + ".retail.txt", "format=TNRUN002\nfields=" + count +
                "\ninput_sha256=" + BitConverter.ToString(traceHash).Replace("-", "").ToLowerInvariant() +
                "\ncore_sha256=" + BitConverter.ToString(coreHash).Replace("-", "").ToLowerInvariant() +
                "\ngame_memory_writes=false\n");
            Dispose();
        }

        public void Dispose()
        {
            if (disposed) return;
            if (installed)
            {
                ram.Enter();
                try { Marshal.WriteIntPtr(cpu, 0x92598, IntPtr.Zero); }
                finally { ram.Exit(); }
            }
            retail_probe_end(); stream.Dispose(); disposed = true;
        }
    }
}
