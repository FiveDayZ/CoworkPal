using System.Diagnostics;
using System.Globalization;
using System.Security.Cryptography;
using System.Text;
using LibreHardwareMonitor.Hardware;
using Microsoft.Win32;

return await HardwareMonitorProgram.RunAsync(args);

internal static class HardwareMonitorProgram
{
    private static readonly TimeSpan SampleInterval = TimeSpan.FromSeconds(2);
    private static readonly Version RequiredPawnIoVersion = new(2, 2, 0, 0);
    private const string PawnIoInstallerSha256 =
        "1F519A22E47187F70A1379A48CA604981C4FCF694F4E65B734AAA74A9FBA3032";

    public static async Task<int> RunAsync(string[] args)
    {
        if (args.Contains("--validate-pawnio-installer", StringComparer.OrdinalIgnoreCase))
        {
            string? installerPath = ResolvePawnIoInstallerPath();
            return installerPath is not null && HasExpectedPawnIoInstallerHash(installerPath)
                ? 0
                : 4;
        }

        string? outputPath = ArgumentValue(args, "--output");
        bool validateOutputPath = args.Contains(
            "--validate-output-path",
            StringComparer.OrdinalIgnoreCase);
        if (validateOutputPath)
        {
            return !string.IsNullOrWhiteSpace(outputPath) && IsAllowedOutputPath(outputPath)
                ? 0
                : 2;
        }

        bool runOnce = args.Contains("--once", StringComparer.OrdinalIgnoreCase);
        int? parentPid = ParsePositiveInt(ArgumentValue(args, "--parent-pid"));
        if (string.IsNullOrWhiteSpace(outputPath)
            || !IsAllowedOutputPath(outputPath)
            || (!runOnce && parentPid is null))
        {
            return 2;
        }

        int? prerequisiteExitCode = EnsurePawnIo(outputPath);
        if (prerequisiteExitCode is not null)
        {
            return prerequisiteExitCode.Value;
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
            string pawnIoVersion = GetPawnIoVersion()?.ToString() ?? "unavailable";

            do
            {
                if (parent is not null && HasExited(parent))
                {
                    break;
                }

                SensorSample sample = ReadSensors(computer, pawnIoVersion);
                WriteSampleAtomically(outputPath, sample);
                TryWriteCpuDiagnostic(outputPath, sample);
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

    private static SensorSample ReadSensors(Computer computer, string pawnIoVersion)
    {
        TemperatureCandidate cpu = new();
        TemperatureCandidate gpu = new();
        List<string> cpuDiagnostics = [$"pawnio='{pawnIoVersion}'"];
        foreach (IHardware hardware in computer.Hardware)
        {
            UpdateHardware(hardware, cpu, gpu, cpuDiagnostics);
        }

        return new SensorSample(
            cpu.ResolveAverage(),
            gpu.ResolveMaximum(),
            string.Join("; ", cpuDiagnostics));
    }

    private static void UpdateHardware(
        IHardware hardware,
        TemperatureCandidate cpu,
        TemperatureCandidate gpu,
        List<string> cpuDiagnostics)
    {
        hardware.Update();
        if (hardware.HardwareType == HardwareType.Cpu)
        {
            AddCpuDiagnostic(cpuDiagnostics, $"hardware='{hardware.Name}'");
        }
        foreach (ISensor sensor in hardware.Sensors)
        {
            if (sensor.SensorType != SensorType.Temperature)
            {
                continue;
            }

            if (hardware.HardwareType == HardwareType.Cpu)
            {
                AddCpuDiagnostic(
                    cpuDiagnostics,
                    $"sensor='{sensor.Name}', value={JsonNumber(sensor.Value)}");
                if (!IsPlausible(sensor.Value))
                {
                    continue;
                }
                cpu.Add(CpuSensorScore(sensor.Name), sensor.Value!.Value);
            }
            else if (hardware.HardwareType is HardwareType.GpuAmd
                     or HardwareType.GpuIntel
                     or HardwareType.GpuNvidia)
            {
                if (!IsPlausible(sensor.Value))
                {
                    continue;
                }
                gpu.Add(GpuSensorScore(sensor.Name), sensor.Value!.Value);
            }
        }

        foreach (IHardware child in hardware.SubHardware)
        {
            UpdateHardware(child, cpu, gpu, cpuDiagnostics);
        }
    }

    private static void AddCpuDiagnostic(List<string> diagnostics, string value)
    {
        if (diagnostics.Count < 32)
        {
            diagnostics.Add(value);
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

    private static void TryWriteCpuDiagnostic(string outputPath, SensorSample sample)
    {
        string diagnosticPath = outputPath + ".diagnostic.log";
        try
        {
            if (sample.CpuTemperatureCelsius is not null)
            {
                File.Delete(diagnosticPath);
                return;
            }

            string detail = string.IsNullOrWhiteSpace(sample.CpuDiagnostic)
                ? "no CPU hardware or temperature sensors were reported"
                : sample.CpuDiagnostic;
            WriteDiagnosticIfChanged(diagnosticPath, "CPU temperature unavailable: " + detail);
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error);
        }
    }

    private static int? EnsurePawnIo(string outputPath)
    {
        Version? installedVersion = GetPawnIoVersion();
        if (installedVersion is not null && installedVersion >= RequiredPawnIoVersion)
        {
            return null;
        }

        string? installerPath = ResolvePawnIoInstallerPath();
        if (installerPath is null)
        {
            TryWriteDiagnostic(
                outputPath,
                "PawnIO 2.2.0 is required for CPU sensor access, but the bundled installer was not found.");
            return 4;
        }
        if (!HasExpectedPawnIoInstallerHash(installerPath))
        {
            TryWriteDiagnostic(
                outputPath,
                "The bundled PawnIO installer failed its integrity check and was not executed.");
            return 4;
        }

        try
        {
            using Process? installer = Process.Start(new ProcessStartInfo
            {
                FileName = installerPath,
                Arguments = "-install -silent",
                UseShellExecute = false,
                CreateNoWindow = true,
                WindowStyle = ProcessWindowStyle.Hidden,
            });
            if (installer is null)
            {
                throw new InvalidOperationException("failed to start the bundled PawnIO installer");
            }

            installer.WaitForExit();
            if (installer.ExitCode == 0)
            {
                return null;
            }
            if (installer.ExitCode == 3010)
            {
                TryWriteDiagnostic(
                    outputPath,
                    "PawnIO was installed successfully, but Windows must be restarted before CPU temperature is available.");
                return 5;
            }

            throw new InvalidOperationException(
                $"bundled PawnIO installer exited with code {installer.ExitCode}");
        }
        catch (Exception error)
        {
            TryWriteError(outputPath, error);
            return 4;
        }
    }

    private static Version? GetPawnIoVersion()
    {
        using RegistryKey registry = RegistryKey.OpenBaseKey(
            RegistryHive.LocalMachine,
            RegistryView.Registry64);
        using RegistryKey? key = registry.OpenSubKey(
            @"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO");
        return Version.TryParse(key?.GetValue("DisplayVersion") as string, out Version? version)
            ? version
            : null;
    }

    private static string? ResolvePawnIoInstallerPath()
    {
        string? directory = Path.GetDirectoryName(Environment.ProcessPath);
        if (directory is null)
        {
            return null;
        }

        return new[]
            {
                "pawnio-setup.exe",
                "pawnio-setup-x86_64-pc-windows-msvc.exe",
            }
            .Select(name => Path.Combine(directory, name))
            .FirstOrDefault(File.Exists);
    }

    private static bool HasExpectedPawnIoInstallerHash(string path)
    {
        try
        {
            return string.Equals(
                Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))),
                PawnIoInstallerSha256,
                StringComparison.Ordinal);
        }
        catch
        {
            return false;
        }
    }

    private static void TryWriteDiagnostic(string outputPath, string diagnostic)
    {
        try
        {
            WriteDiagnosticIfChanged(outputPath + ".diagnostic.log", diagnostic);
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error);
        }
    }

    private static void WriteDiagnosticIfChanged(string path, string diagnostic)
    {
        if (!File.Exists(path)
            || !string.Equals(File.ReadAllText(path), diagnostic, StringComparison.Ordinal))
        {
            File.WriteAllText(path, diagnostic, new UTF8Encoding(false));
        }
    }

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
        try
        {
            string fullPath = Path.GetFullPath(outputPath);
            string allowedDirectory = Path.GetFullPath(Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData),
                "CoworkPal",
                "HardwareMonitor"));
            string? directory = Path.GetDirectoryName(fullPath);
            string fileName = Path.GetFileName(fullPath);
            return directory is not null
                && string.Equals(
                    Path.TrimEndingDirectorySeparator(directory),
                    Path.TrimEndingDirectorySeparator(allowedDirectory),
                    StringComparison.OrdinalIgnoreCase)
                && fileName.StartsWith("coworkpal-hardware-monitor-", StringComparison.Ordinal)
                && fileName.EndsWith(".json", StringComparison.OrdinalIgnoreCase);
        }
        catch (Exception error) when (error is ArgumentException
                                      or NotSupportedException
                                      or PathTooLongException)
        {
            return false;
        }
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
        float? GpuTemperatureCelsius,
        string CpuDiagnostic);
}
