using System.Diagnostics;
using System.Globalization;
using System.Text;
using LibreHardwareMonitor.Hardware;

return await HardwareMonitorProgram.RunAsync(args);

internal static class HardwareMonitorProgram
{
    private static readonly TimeSpan SampleInterval = TimeSpan.FromSeconds(2);

    public static async Task<int> RunAsync(string[] args)
    {
        string? outputPath = ArgumentValue(args, "--output");
        bool runOnce = args.Contains("--once", StringComparer.OrdinalIgnoreCase);
        int? parentPid = ParsePositiveInt(ArgumentValue(args, "--parent-pid"));
        if (string.IsNullOrWhiteSpace(outputPath)
            || !IsAllowedOutputPath(outputPath)
            || (!runOnce && parentPid is null))
        {
            return 2;
        }

        Process? parent = null;
        if (parentPid is not null)
        {
            try
            {
                parent = Process.GetProcessById(parentPid.Value);
            }
            catch (ArgumentException)
            {
                return 3;
            }
        }

        Computer computer = new()
        {
            IsCpuEnabled = true,
            IsGpuEnabled = true,
        };
        try
        {
            computer.Open();

            do
            {
                if (parent is not null && HasExited(parent))
                {
                    break;
                }

                SensorSample sample = ReadSensors(computer);
                WriteSampleAtomically(outputPath, sample);
                if (runOnce)
                {
                    break;
                }

                await Task.Delay(SampleInterval);
            }
            while (true);

            return 0;
        }
        catch (Exception error)
        {
            TryWriteError(outputPath, error);
            return 1;
        }
        finally
        {
            computer.Close();
            parent?.Dispose();
        }
    }

    private static SensorSample ReadSensors(Computer computer)
    {
        TemperatureCandidate cpu = new();
        TemperatureCandidate gpu = new();
        foreach (IHardware hardware in computer.Hardware)
        {
            UpdateHardware(hardware, cpu, gpu);
        }

        return new SensorSample(cpu.ResolveAverage(), gpu.ResolveMaximum());
    }

    private static void UpdateHardware(
        IHardware hardware,
        TemperatureCandidate cpu,
        TemperatureCandidate gpu)
    {
        hardware.Update();
        foreach (ISensor sensor in hardware.Sensors)
        {
            if (sensor.SensorType != SensorType.Temperature || !IsPlausible(sensor.Value))
            {
                continue;
            }

            if (hardware.HardwareType == HardwareType.Cpu)
            {
                cpu.Add(CpuSensorScore(sensor.Name), sensor.Value!.Value);
            }
            else if (hardware.HardwareType is HardwareType.GpuAmd
                     or HardwareType.GpuIntel
                     or HardwareType.GpuNvidia)
            {
                gpu.Add(GpuSensorScore(sensor.Name), sensor.Value!.Value);
            }
        }

        foreach (IHardware child in hardware.SubHardware)
        {
            UpdateHardware(child, cpu, gpu);
        }
    }

    private static int CpuSensorScore(string name)
    {
        string value = name.ToLowerInvariant();
        if (value.Contains("core average")) return 5;
        if (value.Contains("package")) return 4;
        if (value.Contains("tctl") || value.Contains("tdie")) return 3;
        if (value.Contains("core max")) return 2;
        return value.Contains("core") ? 1 : 0;
    }

    private static int GpuSensorScore(string name)
    {
        string value = name.ToLowerInvariant();
        if (value.Contains("gpu core")) return 5;
        if (value.Contains("gpu temperature") || value.Contains("gpu edge")) return 4;
        if (value.Contains("hot spot") || value.Contains("hotspot")) return 2;
        return 0;
    }

    private static bool IsPlausible(float? value) =>
        value is > 0 and <= 125 && float.IsFinite(value.Value);

    private static void WriteSampleAtomically(string outputPath, SensorSample sample)
    {
        string fullPath = Path.GetFullPath(outputPath);
        string? directory = Path.GetDirectoryName(fullPath);
        if (!string.IsNullOrEmpty(directory))
        {
            Directory.CreateDirectory(directory);
        }

        string temporaryPath = fullPath + ".tmp";
        string timestamp = DateTimeOffset.UtcNow
            .ToUnixTimeMilliseconds()
            .ToString(CultureInfo.InvariantCulture);
        string payload = string.Concat(
            "{\"timestampMs\":", timestamp,
            ",\"cpuTemperatureCelsius\":", JsonNumber(sample.CpuTemperatureCelsius),
            ",\"gpuTemperatureCelsius\":", JsonNumber(sample.GpuTemperatureCelsius),
            "}");
        File.WriteAllText(temporaryPath, payload, new UTF8Encoding(false));
        File.Move(temporaryPath, fullPath, true);
        File.Delete(fullPath + ".error.log");
    }

    private static string JsonNumber(float? value) =>
        value?.ToString("0.###", CultureInfo.InvariantCulture) ?? "null";

    private static bool HasExited(Process process)
    {
        try
        {
            return process.HasExited;
        }
        catch
        {
            return true;
        }
    }

    private static bool IsAllowedOutputPath(string outputPath)
    {
        string fullPath = Path.GetFullPath(outputPath);
        string temporaryRoot = Path.TrimEndingDirectorySeparator(
            Path.GetFullPath(Path.GetTempPath())) + Path.DirectorySeparatorChar;
        string fileName = Path.GetFileName(fullPath);
        return fullPath.StartsWith(temporaryRoot, StringComparison.OrdinalIgnoreCase)
            && fileName.StartsWith("coworkpal-hardware-monitor-", StringComparison.Ordinal)
            && fileName.EndsWith(".json", StringComparison.OrdinalIgnoreCase);
    }

    private static void TryWriteError(string outputPath, Exception error)
    {
        try
        {
            File.WriteAllText(outputPath + ".error.log", error.ToString(), new UTF8Encoding(false));
        }
        catch (Exception logError)
        {
            Console.Error.WriteLine(error);
            Console.Error.WriteLine(logError);
        }
    }

    private static string? ArgumentValue(string[] args, string name)
    {
        for (int index = 0; index < args.Length; index++)
        {
            string argument = args[index];
            if (argument.StartsWith(name + "=", StringComparison.OrdinalIgnoreCase))
            {
                return argument[(name.Length + 1)..].Trim('"');
            }
            if (argument.Equals(name, StringComparison.OrdinalIgnoreCase) && index + 1 < args.Length)
            {
                return args[index + 1].Trim('"');
            }
        }
        return null;
    }

    private static int? ParsePositiveInt(string? value) =>
        int.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out int parsed)
        && parsed > 0
            ? parsed
            : null;

    private sealed class TemperatureCandidate
    {
        private int bestScore = -1;
        private readonly List<float> values = [];

        public void Add(int score, float value)
        {
            if (score < bestScore)
            {
                return;
            }
            if (score > bestScore)
            {
                bestScore = score;
                values.Clear();
            }
            values.Add(value);
        }

        public float? ResolveAverage() =>
            values.Count == 0 ? null : values.Average();

        public float? ResolveMaximum() =>
            values.Count == 0 ? null : values.Max();
    }

    private readonly record struct SensorSample(
        float? CpuTemperatureCelsius,
        float? GpuTemperatureCelsius);
}
