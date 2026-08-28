param(
  [ValidateSet("create", "read")] [string] $Operation = "create",
  [Parameter(Mandatory = $true)] [string] $ParentPath,
  [Parameter(Mandatory = $true)] [string] $LeafName,
  [Parameter(Mandatory = $true)] [string] $ExpectedVolumeSerial,
  [Parameter(Mandatory = $true)] [string] $ExpectedFileIndex,
  [UInt64] $MaximumBytes = 0,
  [switch] $AllowMissing,
  [string] $FenceLeafNames = "",
  [string] $FenceByteLengths = "",
  [string] $FenceSha256 = ""
)

$source = @'
using System;
using System.ComponentModel;
using System.IO;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Collections.Generic;
using Microsoft.Win32.SafeHandles;

public static class SyndocalSafeCreate
{
    private const uint FILE_READ_ATTRIBUTES = 0x00000080;
    private const uint FILE_READ_DATA = 0x00000001;
    private const uint FILE_LIST_DIRECTORY = 0x00000001;
    private const uint FILE_WRITE_DATA = 0x00000002;
    private const uint DELETE = 0x00010000;
    private const uint SYNCHRONIZE = 0x00100000;
    private const uint FILE_SHARE_READ = 0x00000001;
    private const uint FILE_SHARE_WRITE = 0x00000002;
    private const uint FILE_SHARE_DELETE = 0x00000004;
    private const uint OPEN_EXISTING = 3;
    private const uint FILE_ATTRIBUTE_NORMAL = 0x00000080;
    private const uint FILE_FLAG_BACKUP_SEMANTICS = 0x02000000;
    private const uint FILE_FLAG_OPEN_REPARSE_POINT = 0x00200000;
    private const uint FILE_ATTRIBUTE_REPARSE_POINT = 0x00000400;
    private const uint FILE_CREATE = 2;
    private const uint FILE_OPEN = 1;
    private const uint FILE_NON_DIRECTORY_FILE = 0x00000040;
    private const uint FILE_SYNCHRONOUS_IO_NONALERT = 0x00000020;
    private const uint FILE_OPEN_FOR_BACKUP_INTENT = 0x00004000;
    private const uint OBJ_CASE_INSENSITIVE = 0x00000040;
    private const uint OBJ_DONT_REPARSE = 0x00001000;
    private const int STATUS_OBJECT_NAME_NOT_FOUND = unchecked((int)0xC0000034);

    [StructLayout(LayoutKind.Sequential, Pack = 1)]
    private struct BY_HANDLE_FILE_INFORMATION
    {
        public uint FileAttributes;
        public long CreationTime;
        public long LastAccessTime;
        public long LastWriteTime;
        public uint VolumeSerialNumber;
        public uint FileSizeHigh;
        public uint FileSizeLow;
        public uint NumberOfLinks;
        public uint FileIndexHigh;
        public uint FileIndexLow;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct FILE_ATTRIBUTE_TAG_INFO
    {
        public uint FileAttributes;
        public uint ReparseTag;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct UNICODE_STRING
    {
        public ushort Length;
        public ushort MaximumLength;
        public IntPtr Buffer;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct OBJECT_ATTRIBUTES
    {
        public int Length;
        public IntPtr RootDirectory;
        public IntPtr ObjectName;
        public uint Attributes;
        public IntPtr SecurityDescriptor;
        public IntPtr SecurityQualityOfService;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct IO_STATUS_BLOCK
    {
        public IntPtr Status;
        public IntPtr Information;
    }

    private enum FILE_INFO_BY_HANDLE_CLASS
    {
        FileDispositionInfo = 4,
        FileAttributeTagInfo = 9,
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct FILE_DISPOSITION_INFO
    {
        [MarshalAs(UnmanagedType.Bool)]
        public bool DeleteFile;
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafeFileHandle CreateFile(
        string fileName,
        uint desiredAccess,
        uint shareMode,
        IntPtr securityAttributes,
        uint creationDisposition,
        uint flagsAndAttributes,
        IntPtr templateFile);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetFileInformationByHandle(SafeFileHandle file, out BY_HANDLE_FILE_INFORMATION information);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetFileInformationByHandleEx(
        SafeFileHandle file,
        FILE_INFO_BY_HANDLE_CLASS fileInformationClass,
        out FILE_ATTRIBUTE_TAG_INFO fileInformation,
        uint bufferSize);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool SetFileInformationByHandle(
        SafeFileHandle file,
        FILE_INFO_BY_HANDLE_CLASS fileInformationClass,
        ref FILE_DISPOSITION_INFO fileInformation,
        uint bufferSize);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool WriteFile(
        SafeFileHandle file,
        byte[] buffer,
        uint numberOfBytesToWrite,
        out uint numberOfBytesWritten,
        IntPtr overlapped);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool ReadFile(
        SafeFileHandle file,
        byte[] buffer,
        uint numberOfBytesToRead,
        out uint numberOfBytesRead,
        IntPtr overlapped);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool FlushFileBuffers(SafeFileHandle file);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool GetVolumeInformation(
        string rootPathName,
        StringBuilder volumeNameBuffer,
        uint volumeNameSize,
        out uint volumeSerialNumber,
        out uint maximumComponentLength,
        out uint fileSystemFlags,
        StringBuilder fileSystemNameBuffer,
        uint fileSystemNameSize);

    [DllImport("ntdll.dll")]
    private static extern int NtCreateFile(
        out IntPtr fileHandle,
        uint desiredAccess,
        ref OBJECT_ATTRIBUTES objectAttributes,
        out IO_STATUS_BLOCK ioStatusBlock,
        IntPtr allocationSize,
        uint fileAttributes,
        uint shareAccess,
        uint createDisposition,
        uint createOptions,
        IntPtr eaBuffer,
        uint eaLength);

    public static void CreateAndWrite(string parentPath, string leafName, byte[] bytes, string expectedVolumeSerial, string expectedFileIndex, string[] fenceLeafNames, string[] fenceByteLengths, string[] fenceSha256)
    {
        ValidateOutputBoundary(parentPath, leafName);
        if (bytes == null) throw new InvalidOperationException("input bytes are missing");
        ulong expectedVolume;
        ulong expectedIndex;
        if (!UInt64.TryParse(expectedVolumeSerial, out expectedVolume) || !UInt64.TryParse(expectedFileIndex, out expectedIndex))
            throw new InvalidOperationException("parent identity is invalid");
        if ((ulong)bytes.Length > UInt32.MaxValue)
            throw new InvalidOperationException("input bytes exceed the native write limit");

        using (var parent = CreateFile(parentPath, FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY,
                   FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero,
                   OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, IntPtr.Zero))
        {
            if (parent.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error(), "opening output parent failed");
            AssertLocalNtfs(parentPath);
            AssertParentStable(parent, parentPath, expectedVolume, expectedIndex, "before native create");

            var fences = OpenAndAssertFences(parent, parentPath, leafName, fenceLeafNames, fenceByteLengths, fenceSha256);
            try
            {

            var leafBuffer = Marshal.StringToHGlobalUni(leafName);
            var unicode = new UNICODE_STRING
            {
                Length = checked((ushort)(leafName.Length * 2)),
                MaximumLength = checked((ushort)(leafName.Length * 2 + 2)),
                Buffer = leafBuffer,
            };
            var unicodePointer = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(UNICODE_STRING)));
            try
            {
                Marshal.StructureToPtr(unicode, unicodePointer, false);
                var objectAttributes = new OBJECT_ATTRIBUTES
                {
                    Length = Marshal.SizeOf(typeof(OBJECT_ATTRIBUTES)),
                    RootDirectory = parent.DangerousGetHandle(),
                    ObjectName = unicodePointer,
                    Attributes = OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
                };
                IntPtr rawHandle;
                IO_STATUS_BLOCK ioStatus;
                var status = NtCreateFile(
                    out rawHandle,
                    FILE_WRITE_DATA | FILE_READ_ATTRIBUTES | DELETE | SYNCHRONIZE,
                    ref objectAttributes,
                    out ioStatus,
                    IntPtr.Zero,
                    FILE_ATTRIBUTE_NORMAL,
                    FILE_SHARE_READ,
                    FILE_CREATE,
                    FILE_NON_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT,
                    IntPtr.Zero,
                    0);
                if (status != 0) throw new InvalidOperationException("NtCreateFile failed: 0x" + status.ToString("X8"));
                using (var child = new SafeFileHandle(rawHandle, true))
                {
                    try
                    {
                        AssertChildRegular(child, "after native create");
                        AssertParentStable(parent, parentPath, expectedVolume, expectedIndex, "after native create");
                        AssertLexicalChild(child, Path.Combine(parentPath, leafName), "before write");
                        WriteExact(child, bytes, bytes.Length, "output write");
                        AssertParentStable(parent, parentPath, expectedVolume, expectedIndex, "after write");
                        AssertLexicalChild(child, Path.Combine(parentPath, leafName), "after write");
                        if (!FlushFileBuffers(child))
                            throw new Win32Exception(Marshal.GetLastWin32Error(), "flushing output failed");
                        AssertParentStable(parent, parentPath, expectedVolume, expectedIndex, "after flush");
                        AssertLexicalChild(child, Path.Combine(parentPath, leafName), "after flush");
                        AssertChildLength(child, (ulong)bytes.Length);
                    }
                    catch (Exception error)
                    {
                        try
                        {
                            DeletePending(child);
                        }
                        catch (Exception cleanupError)
                        {
                            throw new InvalidOperationException(error.Message + "; created output cleanup failed: " + cleanupError.Message, error);
                        }
                        throw;
                    }
                }
            }
            finally
            {
                Marshal.FreeHGlobal(unicodePointer);
                Marshal.FreeHGlobal(leafBuffer);
            }
            }
            finally
            {
                foreach (var fence in fences) fence.Dispose();
            }
        }
    }

    public static byte[] ReadRegular(string parentPath, string leafName, string expectedVolumeSerial, string expectedFileIndex, ulong maximumBytes, bool allowMissing)
    {
        ValidateOutputBoundary(parentPath, leafName);
        ulong expectedVolume;
        ulong expectedIndex;
        if (!UInt64.TryParse(expectedVolumeSerial, out expectedVolume) || !UInt64.TryParse(expectedFileIndex, out expectedIndex))
            throw new InvalidOperationException("parent identity is invalid");
        if (maximumBytes > Int32.MaxValue) throw new InvalidOperationException("input bytes exceed the native read limit");

        using (var parent = CreateFile(parentPath, FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY,
                   FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero,
                   OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, IntPtr.Zero))
        {
            if (parent.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error(), "opening input parent failed");
            AssertLocalNtfs(parentPath);
            AssertParentStable(parent, parentPath, expectedVolume, expectedIndex, "before native read");
            var child = OpenExistingChild(parent, leafName, allowMissing, "input");
            if (child == null) return null;
            using (child)
            {
                AssertChildRegular(child, "before native read");
                AssertLexicalChild(child, Path.Combine(parentPath, leafName), "before native read");
                AssertParentStable(parent, parentPath, expectedVolume, expectedIndex, "before native read bytes");
                var length = ChildLength(child);
                if (length > maximumBytes) throw new InvalidOperationException("input exceeds the bounded native read size");
                var bytes = ReadExact(child, checked((int)length), "input read");
                AssertChildLength(child, length);
                AssertLexicalChild(child, Path.Combine(parentPath, leafName), "after native read");
                AssertParentStable(parent, parentPath, expectedVolume, expectedIndex, "after native read");
                return bytes;
            }
        }
    }

    public static string[] SplitFenceList(string value)
    {
        if (String.IsNullOrEmpty(value)) return new string[0];
        return value.Split(new[] { '|' }, StringSplitOptions.None);
    }

    private static List<SafeFileHandle> OpenAndAssertFences(SafeFileHandle parent, string parentPath, string outputLeafName, string[] fenceLeafNames, string[] fenceByteLengths, string[] fenceSha256)
    {
        fenceLeafNames = fenceLeafNames ?? new string[0];
        fenceByteLengths = fenceByteLengths ?? new string[0];
        fenceSha256 = fenceSha256 ?? new string[0];
        if (fenceLeafNames.Length != fenceByteLengths.Length || fenceLeafNames.Length != fenceSha256.Length)
            throw new InvalidOperationException("publication fence arrays must have identical lengths");
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        seen.Add(outputLeafName);
        var fences = new List<SafeFileHandle>();
        try
        {
            for (var index = 0; index < fenceLeafNames.Length; index++)
            {
                var leafName = fenceLeafNames[index];
                ValidateOutputBoundary(parentPath, leafName);
                if (!seen.Add(leafName)) throw new InvalidOperationException("publication fences must name distinct non-output leaves");
                ulong expectedLength;
                if (!UInt64.TryParse(fenceByteLengths[index], out expectedLength) || expectedLength > Int32.MaxValue)
                    throw new InvalidOperationException("publication fence length is invalid");
                var expectedHash = fenceSha256[index] ?? String.Empty;
                if (expectedHash.Length != 64 || !IsLowerHex(expectedHash))
                    throw new InvalidOperationException("publication fence SHA-256 is invalid");
                var child = OpenExistingChild(parent, leafName, false, "publication fence");
                try
                {
                    AssertChildRegular(child, "publication fence");
                    AssertLexicalChild(child, Path.Combine(parentPath, leafName), "publication fence");
                    if (ChildLength(child) != expectedLength)
                        throw new InvalidOperationException("publication fence byte length changed for " + leafName);
                    var actualHash = Hex(SHA256.Create().ComputeHash(ReadExact(child, checked((int)expectedLength), "publication fence read")));
                    if (!String.Equals(actualHash, expectedHash, StringComparison.Ordinal))
                        throw new InvalidOperationException("publication fence SHA-256 changed for " + leafName);
                    AssertChildLength(child, expectedLength);
                    AssertLexicalChild(child, Path.Combine(parentPath, leafName), "publication fence after read");
                    fences.Add(child);
                }
                catch
                {
                    child.Dispose();
                    throw;
                }
            }
            return fences;
        }
        catch
        {
            foreach (var fence in fences) fence.Dispose();
            throw;
        }
    }

    private static SafeFileHandle OpenExistingChild(SafeFileHandle parent, string leafName, bool allowMissing, string label)
    {
        var leafBuffer = Marshal.StringToHGlobalUni(leafName);
        var unicode = new UNICODE_STRING
        {
            Length = checked((ushort)(leafName.Length * 2)),
            MaximumLength = checked((ushort)(leafName.Length * 2 + 2)),
            Buffer = leafBuffer,
        };
        var unicodePointer = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(UNICODE_STRING)));
        try
        {
            Marshal.StructureToPtr(unicode, unicodePointer, false);
            var objectAttributes = new OBJECT_ATTRIBUTES
            {
                Length = Marshal.SizeOf(typeof(OBJECT_ATTRIBUTES)),
                RootDirectory = parent.DangerousGetHandle(),
                ObjectName = unicodePointer,
                Attributes = OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
            };
            IntPtr rawHandle;
            IO_STATUS_BLOCK ioStatus;
            var status = NtCreateFile(
                out rawHandle,
                FILE_READ_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                ref objectAttributes,
                out ioStatus,
                IntPtr.Zero,
                FILE_ATTRIBUTE_NORMAL,
                FILE_SHARE_READ,
                FILE_OPEN,
                FILE_NON_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT,
                IntPtr.Zero,
                0);
            if (status == STATUS_OBJECT_NAME_NOT_FOUND && allowMissing) return null;
            if (status != 0) throw new InvalidOperationException("NtCreateFile " + label + " failed: 0x" + status.ToString("X8"));
            return new SafeFileHandle(rawHandle, true);
        }
        finally
        {
            Marshal.FreeHGlobal(unicodePointer);
            Marshal.FreeHGlobal(leafBuffer);
        }
    }

    private static byte[] ReadExact(SafeFileHandle child, int length, string phase)
    {
        if (length < 0) throw new InvalidOperationException("native read length is invalid");
        var bytes = new byte[length];
        if (length == 0) return bytes;
        uint read;
        if (!ReadFile(child, bytes, (uint)length, out read, IntPtr.Zero))
            throw new Win32Exception(Marshal.GetLastWin32Error(), phase + " failed");
        if (read != (uint)length)
            throw new IOException(phase + " was partial or truncated (" + read + "/" + length + " bytes)");
        return bytes;
    }

    private static ulong ChildLength(SafeFileHandle child)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (!GetFileInformationByHandle(child, out information))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "reading child length failed");
        return ((ulong)information.FileSizeHigh << 32) | information.FileSizeLow;
    }

    private static bool IsLowerHex(string text)
    {
        foreach (var character in text)
        {
            if (!((character >= '0' && character <= '9') || (character >= 'a' && character <= 'f')))
                return false;
        }
        return true;
    }

    private static string Hex(byte[] bytes)
    {
        var builder = new StringBuilder(bytes.Length * 2);
        foreach (var value in bytes) builder.Append(value.ToString("x2"));
        return builder.ToString();
    }

    private static void ValidateOutputBoundary(string parentPath, string leafName)
    {
        if (String.IsNullOrEmpty(leafName) || leafName == "." || leafName == ".." || leafName.IndexOfAny(new[] { '\\', '/', ':' }) >= 0)
            throw new InvalidOperationException("leaf name must be a single path component without separators or alternate data streams");
        if (leafName.EndsWith(".", StringComparison.Ordinal) || leafName.EndsWith(" ", StringComparison.Ordinal))
            throw new InvalidOperationException("leaf name must not end with a dot or space");
        var deviceName = leafName;
        var dot = deviceName.IndexOf('.');
        if (dot >= 0) deviceName = deviceName.Substring(0, dot);
        deviceName = deviceName.ToUpperInvariant();
        if (deviceName == "CON" || deviceName == "PRN" || deviceName == "AUX" || deviceName == "NUL" || deviceName == "CLOCK$" ||
            (deviceName.Length == 4 && (deviceName.StartsWith("COM", StringComparison.Ordinal) || deviceName.StartsWith("LPT", StringComparison.Ordinal)) && deviceName[3] >= '1' && deviceName[3] <= '9'))
            throw new InvalidOperationException("leaf name must not be a Windows device name");
        var root = Path.GetPathRoot(parentPath);
        if (String.IsNullOrEmpty(root) || root.Length != 3 || root[1] != ':' || root[2] != '\\')
            throw new InvalidOperationException("output supports only local drive paths on NTFS");
        if (Path.Combine(parentPath, leafName).Length > 259)
            throw new InvalidOperationException("output path exceeds the supported local NTFS boundary");
    }

    private static void AssertParentIdentity(SafeFileHandle parent, ulong expectedVolume, ulong expectedIndex)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (!GetFileInformationByHandle(parent, out information))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "reading output parent identity failed");
        var index = ((ulong)information.FileIndexHigh << 32) | information.FileIndexLow;
        if (information.VolumeSerialNumber != expectedVolume || index != expectedIndex)
            throw new InvalidOperationException("output parent identity changed before native create (expected " + expectedVolume + ":" + expectedIndex + ", actual " + information.VolumeSerialNumber + ":" + index + ")");
    }

    private static void AssertLocalNtfs(string parentPath)
    {
        var root = Path.GetPathRoot(parentPath);
        if (String.IsNullOrEmpty(root) || root.StartsWith("\\\\", StringComparison.Ordinal) || !root.EndsWith("\\", StringComparison.Ordinal))
            throw new InvalidOperationException("output supports only local drive paths on NTFS");
        var volumeName = new StringBuilder(260);
        var fileSystemName = new StringBuilder(32);
        uint serial;
        uint maximumComponentLength;
        uint fileSystemFlags;
        if (!GetVolumeInformation(root, volumeName, (uint)volumeName.Capacity, out serial, out maximumComponentLength, out fileSystemFlags, fileSystemName, (uint)fileSystemName.Capacity))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "reading output filesystem identity failed");
        if (!String.Equals(fileSystemName.ToString(), "NTFS", StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("output filesystem must be NTFS; refusing an unsupported volume");
    }

    private static void AssertParentStable(SafeFileHandle parent, string parentPath, ulong expectedVolume, ulong expectedIndex, string phase)
    {
        AssertParentIdentity(parent, expectedVolume, expectedIndex);
        var attributes = new FILE_ATTRIBUTE_TAG_INFO();
        if (!GetFileInformationByHandleEx(parent, FILE_INFO_BY_HANDLE_CLASS.FileAttributeTagInfo, out attributes, (uint)Marshal.SizeOf(typeof(FILE_ATTRIBUTE_TAG_INFO))))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "reading output parent attributes " + phase + " failed");
        if ((attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0)
            throw new InvalidOperationException("output parent is a reparse point " + phase);

        using (var lexicalParent = CreateFile(parentPath, FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY,
                   FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero,
                   OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, IntPtr.Zero))
        {
            if (lexicalParent.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error(), "opening lexical output parent " + phase + " failed");
            AssertParentIdentity(lexicalParent, expectedVolume, expectedIndex);
            var lexicalAttributes = new FILE_ATTRIBUTE_TAG_INFO();
            if (!GetFileInformationByHandleEx(lexicalParent, FILE_INFO_BY_HANDLE_CLASS.FileAttributeTagInfo, out lexicalAttributes, (uint)Marshal.SizeOf(typeof(FILE_ATTRIBUTE_TAG_INFO))))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "reading lexical output parent attributes " + phase + " failed");
            if ((lexicalAttributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0)
                throw new InvalidOperationException("lexical output parent is a reparse point " + phase);
        }
    }

    private static void AssertLexicalChild(SafeFileHandle child, string childPath, string phase)
    {
        BY_HANDLE_FILE_INFORMATION childInformation;
        if (!GetFileInformationByHandle(child, out childInformation))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "reading created output identity " + phase + " failed");
        var childIndex = ((ulong)childInformation.FileIndexHigh << 32) | childInformation.FileIndexLow;
        using (var lexicalChild = CreateFile(childPath, FILE_READ_ATTRIBUTES,
                   FILE_SHARE_READ | FILE_SHARE_DELETE, IntPtr.Zero,
                   OPEN_EXISTING, FILE_FLAG_OPEN_REPARSE_POINT, IntPtr.Zero))
        {
            if (lexicalChild.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error(), "opening lexical output " + phase + " failed");
            BY_HANDLE_FILE_INFORMATION lexicalInformation;
            if (!GetFileInformationByHandle(lexicalChild, out lexicalInformation))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "reading lexical output identity " + phase + " failed");
            var lexicalIndex = ((ulong)lexicalInformation.FileIndexHigh << 32) | lexicalInformation.FileIndexLow;
            if (childInformation.VolumeSerialNumber != lexicalInformation.VolumeSerialNumber || childIndex != lexicalIndex)
                throw new InvalidOperationException("lexical output identity changed " + phase);
            var attributes = new FILE_ATTRIBUTE_TAG_INFO();
            if (!GetFileInformationByHandleEx(lexicalChild, FILE_INFO_BY_HANDLE_CLASS.FileAttributeTagInfo, out attributes, (uint)Marshal.SizeOf(typeof(FILE_ATTRIBUTE_TAG_INFO))))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "reading lexical output attributes " + phase + " failed");
            if ((attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0)
                throw new InvalidOperationException("lexical output is a reparse point " + phase);
        }
    }

    private static void AssertChildRegular(SafeFileHandle child, string phase)
    {
        var attributes = new FILE_ATTRIBUTE_TAG_INFO();
        if (!GetFileInformationByHandleEx(child, FILE_INFO_BY_HANDLE_CLASS.FileAttributeTagInfo, out attributes, (uint)Marshal.SizeOf(typeof(FILE_ATTRIBUTE_TAG_INFO))))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "reading created output attributes " + phase + " failed");
        if ((attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0)
            throw new InvalidOperationException("created output is a reparse point " + phase);
    }

    private static void WriteExact(SafeFileHandle child, byte[] bytes, int length, string phase)
    {
        if (length < 0 || length > bytes.Length) throw new InvalidOperationException("native write length is invalid");
        if (length == 0) return;
        uint written;
        if (!WriteFile(child, bytes, (uint)length, out written, IntPtr.Zero))
            throw new Win32Exception(Marshal.GetLastWin32Error(), phase + " failed");
        if (written != (uint)length)
            throw new IOException(phase + " was partial (" + written + "/" + length + " bytes)");
    }

    private static void AssertChildLength(SafeFileHandle child, ulong expectedLength)
    {
        BY_HANDLE_FILE_INFORMATION information;
        if (!GetFileInformationByHandle(child, out information))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "reading created output length failed");
        var actualLength = ((ulong)information.FileSizeHigh << 32) | information.FileSizeLow;
        if (actualLength != expectedLength)
            throw new InvalidOperationException("created output length changed (expected " + expectedLength + ", actual " + actualLength + ")");
    }

    private static void DeletePending(SafeFileHandle child)
    {
        var disposition = new FILE_DISPOSITION_INFO { DeleteFile = true };
        if (!SetFileInformationByHandle(child, FILE_INFO_BY_HANDLE_CLASS.FileDispositionInfo, ref disposition, (uint)Marshal.SizeOf(typeof(FILE_DISPOSITION_INFO))))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "marking created output DeletePending failed");
    }

}
'@

try {
  Add-Type -TypeDefinition $source -Language CSharp -ErrorAction Stop
  if ($Operation -eq "read") {
    $readBytes = [SyndocalSafeCreate]::ReadRegular($ParentPath, $LeafName, $ExpectedVolumeSerial, $ExpectedFileIndex, $MaximumBytes, [bool]$AllowMissing)
    if ($null -eq $readBytes) { exit 3 }
    $output = [Console]::OpenStandardOutput()
    $output.Write($readBytes, 0, $readBytes.Length)
    $output.Flush()
    exit 0
  }

  $bytes = New-Object System.Collections.Generic.List[byte]
  $stream = [Console]::OpenStandardInput()
  $buffer = New-Object byte[] 65536
  while (($count = $stream.Read($buffer, 0, $buffer.Length)) -gt 0) {
    for ($index = 0; $index -lt $count; $index++) { $bytes.Add($buffer[$index]) }
  }
  [SyndocalSafeCreate]::CreateAndWrite($ParentPath, $LeafName, $bytes.ToArray(), $ExpectedVolumeSerial, $ExpectedFileIndex, [SyndocalSafeCreate]::SplitFenceList($FenceLeafNames), [SyndocalSafeCreate]::SplitFenceList($FenceByteLengths), [SyndocalSafeCreate]::SplitFenceList($FenceSha256))
  exit 0
} catch {
  [Console]::Error.WriteLine($_.Exception.Message)
  exit 2
}
