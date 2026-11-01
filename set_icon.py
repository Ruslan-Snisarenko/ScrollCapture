import ctypes
from ctypes import wintypes
import struct

def set_exe_icon(exe_path, ico_path):
    with open(ico_path, "rb") as f:
        ico_data = f.read()

    idReserved, idType, idCount = struct.unpack_from("<HHH", ico_data, 0)
    assert idType == 1, "Not an ICO file"

    entries = []
    offset = 6
    for i in range(idCount):
        bWidth, bHeight, bColorCount, bReserved, wPlanes, wBitCount, dwBytesInRes, dwImageOffset = struct.unpack_from("<BBBBHHII", ico_data, offset)
        entries.append({
            "w": bWidth, "h": bHeight, "colors": bColorCount, "res": bReserved,
            "planes": wPlanes, "bitCount": wBitCount, "bytes": dwBytesInRes,
            "offset": dwImageOffset, "id": i + 1
        })
        offset += 16

    kernel32 = ctypes.windll.kernel32
    kernel32.BeginUpdateResourceW.restype = wintypes.HANDLE
    kernel32.BeginUpdateResourceW.argtypes = [wintypes.LPCWSTR, wintypes.BOOL]

    kernel32.UpdateResourceW.restype = wintypes.BOOL
    kernel32.UpdateResourceW.argtypes = [
        wintypes.HANDLE,
        wintypes.LPCWSTR,
        wintypes.LPCWSTR,
        wintypes.WORD,
        wintypes.LPVOID,
        wintypes.DWORD
    ]

    kernel32.EndUpdateResourceW.restype = wintypes.BOOL
    kernel32.EndUpdateResourceW.argtypes = [wintypes.HANDLE, wintypes.BOOL]

    h_update = kernel32.BeginUpdateResourceW(exe_path, False)
    if not h_update:
        print("BeginUpdateResourceW failed:", ctypes.GetLastError())
        return False

    RT_ICON = 3
    RT_GROUP_ICON = 14

    for entry in entries:
        img_data = ico_data[entry["offset"]:entry["offset"] + entry["bytes"]]
        res = kernel32.UpdateResourceW(
            h_update,
            wintypes.LPCWSTR(RT_ICON),
            wintypes.LPCWSTR(entry["id"]),
            1033,
            img_data,
            len(img_data)
        )
        if not res:
            print(f"UpdateResourceW RT_ICON {entry['id']} failed:", ctypes.GetLastError())

    # GRPICONDIR structure:
    # GRPICONDIR: idReserved(2), idType(2), idCount(2)
    # GRPICONDIRENTRY: bWidth(1), bHeight(1), bColorCount(1), bReserved(1), wPlanes(2), wBitCount(2), dwBytesInRes(4), nID(2)
    # NOTE: GRPICONDIRENTRY ends with 2-byte nID, NOT 4-byte dwImageOffset! (Total length = 14 bytes per entry)
    grp_dir = struct.pack("<HHH", idReserved, idType, idCount)
    for entry in entries:
        grp_dir += struct.pack(
            "<BBBBHHIH",
            entry["w"], entry["h"], entry["colors"], entry["res"],
            entry["planes"], entry["bitCount"], entry["bytes"],
            entry["id"] # nID is 2 bytes (H)!
        )

    res = kernel32.UpdateResourceW(
        h_update,
        wintypes.LPCWSTR(RT_GROUP_ICON),
        wintypes.LPCWSTR(1), # Main icon group ID
        1033,
        grp_dir,
        len(grp_dir)
    )
    if not res:
        print("UpdateResourceW RT_GROUP_ICON failed:", ctypes.GetLastError())

    ok = kernel32.EndUpdateResourceW(h_update, False)
    print("EndUpdateResourceW result:", ok)
    return ok

set_exe_icon(
    r"C:\Users\rusua\Desktop\Projects\ScrollCapture_Rust\ScrollCapture.exe",
    r"C:\Users\rusua\Desktop\Projects\ScrollCapture_Rust\app_icon.ico"
)
