"""Exercise the native mobile ABI using Linux fixtures."""
import ctypes
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "tools/cross/apk-ui/seed"
INCLUDES = ROOT / "automation/include"
SHIM = (ROOT / "tools/cross/velnosub-aegisub.lua").read_bytes()


def test_video(abi, workdir):
    video = workdir / "vfr.mkv"
    subprocess.run([
        "ffmpeg", "-v", "error", "-f", "lavfi", "-i", "testsrc2=s=96x64:r=25:d=4",
        "-vf", "select=not(mod(n\\,3))+not(mod(n\\,7))", "-fps_mode", "vfr",
        "-c:v", "libx264", "-g", "10", "-bf", "3", str(video),
    ], check=True)
    frames = json.loads(subprocess.check_output([
        "ffprobe", "-v", "error", "-select_streams", "v:0", "-show_frames",
        "-show_entries", "frame=best_effort_timestamp_time,key_frame", "-of", "json", str(video),
    ]))["frames"]
    expected = [round(float(frame["best_effort_timestamp_time"]) * 1000) for frame in frames]
    expected = [time - expected[0] for time in expected]
    keys = [i for i, frame in enumerate(frames) if frame["key_frame"]]
    h = abi.velnosub_abi_video_open(bytes(video))
    assert h >= 0, abi.velnosub_abi_last_error()
    try:
        times = (ctypes.c_int * len(expected))()
        assert abi.velnosub_abi_video_timeline(h, times, len(times)) == 0
        assert list(times) == expected
        keyframes = (ctypes.c_int * len(keys))()
        assert abi.velnosub_abi_video_keyframes(h, keyframes, len(keyframes)) == len(keys)
        assert list(keyframes) == keys
        source = SEED / "karaskel-dene.ass"
        document = abi.velnosub_abi_open(bytes(source))
        assert document >= 0, abi.velnosub_abi_last_error()
        script = f"""
local function check_timing(subs, sel)
    local width, height = aegisub.video_size()
    assert(width == 96 and height == 64)
    assert(aegisub.ms_from_frame(3) == {expected[3]})
    assert(aegisub.frame_from_ms({expected[3]}) == 3)
    local keys = aegisub.keyframes()
    assert(#keys == {len(keys)} and keys[2] == {keys[1]})
end
aegisub.register_macro("Timing", "", check_timing)
""".encode()
        try:
            result = abi.velnosub_abi_run_script(
                document, 0, SHIM, script, b"Timing", bytes(source), h, 0,
            )
            assert result >= 0, abi.velnosub_abi_last_error()
            abi.velnosub_abi_close(result)
        finally:
            abi.velnosub_abi_close(document)
        buffer = ctypes.create_string_buffer(96 * 64 * 4)

        def read_frame(frame):
            assert abi.velnosub_abi_video_frame(h, frame, buffer, len(buffer)) == len(buffer)
            return buffer.raw

        sequential = [read_frame(frame) for frame in range(len(frames))]
        for frame in [0, len(frames) - 1, 2, 30, 1, 16, 15, 17, 39, 0]:
            assert read_frame(frame) == sequential[frame], frame
        callback = ctypes.CFUNCTYPE(ctypes.c_int, ctypes.c_void_p)(lambda _: 1)
        assert abi.velnosub_abi_video_open_cancel(bytes(video), callback, None) < 0
        assert abi.velnosub_abi_video_open(bytes(workdir / "missing.mp4")) < 0
        assert read_frame(0) == sequential[0]
    finally:
        abi.velnosub_abi_video_close(h)

    cfr = workdir / "cfr.mp4"
    subprocess.run([
        "ffmpeg", "-v", "error", "-f", "lavfi", "-i", "testsrc2=s=96x64:r=25:d=1",
        "-c:v", "libx264", str(cfr),
    ], check=True)
    h = abi.velnosub_abi_video_open(bytes(cfr))
    assert h >= 0, abi.velnosub_abi_last_error()
    try:
        times = (ctypes.c_int * 25)()
        assert abi.velnosub_abi_video_timeline(h, times, len(times)) == 0
        assert list(times) == [i * 40 for i in range(25)]
    finally:
        abi.velnosub_abi_video_close(h)


def macro_names(abi, script):
    buffer = ctypes.create_string_buffer(16384)
    length = abi.velnosub_abi_list_macros(SHIM, script, buffer, len(buffer))
    assert length >= 0, abi.velnosub_abi_last_error()
    return buffer.raw[:length].splitlines()


def test_audio_and_embedded_ass(abi, workdir):
    cache = workdir / "audio-cache"
    cache.mkdir()
    os.environ["VELNOSUB_CACHE"] = str(cache)
    inputs = (("mp3", "libmp3lame"), ("flac", "flac"), ("wv", "wavpack"),
              ("ts", "aac"), ("m4a", "aac"))
    previous = None
    try:
        for extension, codec in inputs:
            path = workdir / f"tone.{extension}"
            subprocess.run([
                "ffmpeg", "-v", "error", "-f", "lavfi", "-i", "sine=f=440:d=0.4",
                "-c:a", codec, str(path),
            ], check=True)
            handle = abi.velnosub_abi_audio_file_open(bytes(path))
            assert handle >= 0, abi.velnosub_abi_last_error()
            if previous is not None:
                abi.velnosub_abi_audio_close(previous)
            previous = handle
        cached = set(cache.iterdir())
        assert len(cached) == 1
        callback = ctypes.CFUNCTYPE(ctypes.c_int, ctypes.c_void_p)(lambda _: 1)
        assert abi.velnosub_abi_audio_file_open_cancel(bytes(path), callback, None) < 0
        assert abi.velnosub_abi_audio_file_open(bytes(workdir / "missing.mp3")) < 0
        assert set(cache.iterdir()) == cached
        info = ctypes.create_string_buffer(256)
        assert abi.velnosub_abi_audio_info(previous, info, len(info)) > 0
        assert b"48000Hz" in info.value
    finally:
        if previous is not None:
            abi.velnosub_abi_audio_close(previous)
    assert not list(cache.iterdir())

    source = SEED / "karaskel-dene.ass"
    container = workdir / "embedded.mkv"
    subprocess.run([
        "ffmpeg", "-v", "error", "-i", str(source), "-map", "0:s:0",
        "-c:s", "copy", str(container),
    ], check=True)
    output = workdir / "extracted.ass"
    assert abi.velnosub_abi_subtitle_extract(bytes(container), 0, bytes(output)) > 0

    def lines(path, prefix):
        return [line for line in path.read_text(encoding="utf-8-sig").splitlines()
                if line.startswith(prefix)]

    for prefix in ("Style:", "Dialogue:"):
        assert lines(source, prefix) and lines(output, prefix) == lines(source, prefix), prefix


def test_automation_and_fonts(abi, workdir):
    font = SEED / "DejaVuSans.ttf"
    os.environ["VELNOSUB_FONT"] = str(font)
    source = workdir / "kara.ass"
    seed = (SEED / "karaskel-dene.ass").read_text()
    source.write_text(
        seed + "\nComment: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,template line,{\\bord2}\n"
    )
    compiler = (INCLUDES / "moonscript.lua").read_bytes()
    moon = (ROOT / "automation/autoload/select-overlaps.moon").read_bytes()
    escaped = "".join(f"\\{byte:03}" for byte in moon).encode()
    wrapped = b'local moon = (function()\n' + compiler + b'\nend)()\nassert(moon.loadstring("' + escaped + b'"))()'
    assert b"Select overlaps" in macro_names(abi, wrapped)

    karaoke = (ROOT / "automation/autoload/kara-templater.lua").read_bytes()
    macro = b"Apply karaoke template"
    assert macro in macro_names(abi, karaoke)
    document = abi.velnosub_abi_open(bytes(source))
    assert document >= 0, abi.velnosub_abi_last_error()
    try:
        before = abi.velnosub_abi_event_count(document)
        failing = b"""
aegisub.register_macro("Fail", "", function(subs, sel)
    subs.delete(sel[1])
    error("rollback")
end)
"""
        assert abi.velnosub_abi_run_macro(document, 0, SHIM, failing, b"Fail", bytes(source)) < 0
        assert b"rollback" in abi.velnosub_abi_last_error()
        assert abi.velnosub_abi_event_count(document) == before
        result = abi.velnosub_abi_run_macro(document, 0, SHIM, karaoke, macro, bytes(source))
        assert result >= 0, abi.velnosub_abi_last_error()
        try:
            assert abi.velnosub_abi_event_count(document) == before
            assert abi.velnosub_abi_event_count(result) > before
            output = workdir / "kara-output.ass"
            assert abi.velnosub_abi_save(result, bytes(output)) == 0
            assert ",fx," in output.read_text()
        finally:
            abi.velnosub_abi_close(result)
        filtered = abi.velnosub_abi_run_filter(document, SHIM, karaoke, b"Karaoke template", bytes(source))
        assert filtered >= 0, abi.velnosub_abi_last_error()
        try:
            assert abi.velnosub_abi_event_count(filtered) > before
            assert abi.velnosub_abi_event_count(document) == before
        finally:
            abi.velnosub_abi_close(filtered)
        assert abi.velnosub_abi_attach_font(document, bytes(font)) == 0
        output = workdir / "font.ass"
        assert abi.velnosub_abi_save(document, bytes(output)) == 0
        assert "[Fonts]" in output.read_text()
        opened = abi.velnosub_abi_open(bytes(output))
        assert opened >= 0, abi.velnosub_abi_last_error()
        try:
            assert abi.velnosub_abi_render_open(opened, 320, 240, bytes(font), b"DejaVu Sans") == 0
        finally:
            abi.velnosub_abi_close(opened)
        srt = workdir / "copy.srt"
        assert abi.velnosub_abi_save(document, bytes(srt)) == 0
        assert b" --> " in srt.read_bytes()
    finally:
        abi.velnosub_abi_close(document)


def test_interactive_macro(abi, workdir):
    document = abi.velnosub_abi_new_document()
    assert document >= 0, abi.velnosub_abi_last_error()
    script = b"""
aegisub.register_macro("Interactive", "", function(subs, selected)
    local index = selected[1]
    local line = subs[index]
    line.text = "pending"
    subs[index] = line
    local button, values = aegisub.dialog.display(
        {{class="edit", name="message", text=""}}, {"Apply"})
    assert(button == "Apply")
    local second, options = aegisub.dialog.display(
        {{class="checkbox", name="confirm", value=true}}, {"Finish"})
    assert(second == "Finish" and options.confirm)
    line.text = values.message
    subs[index] = line
end)
"""
    modules = """
local regex = require("aegisub.re")
assert(#regex.find("çğ", ".") == 2)
local lfs = require("lfs")
assert(lfs.attributes(aegisub.decode_path("?data/moonscript.lua"), "mode") == "file")
local bins = require("luabins")
local ok, value = bins.load(assert(bins.save("module test")))
assert(ok and value == "module test")
aegisub.register_macro("Modules", "", function() end)
"""
    assert macro_names(abi, modules.encode()) == [b"Modules"]
    result, token = ctypes.c_int(-1), ctypes.c_int(-1)
    def start():
        return abi.velnosub_abi_macro_start(
            document, 0, SHIM, script, b"Interactive", b"", -1,
            ctypes.byref(token), ctypes.byref(result),
        )
    try:
        assert start() == 1, abi.velnosub_abi_last_error()
        assert abi.velnosub_abi_dialog_count(result.value) == 1
        abi.velnosub_abi_macro_abort(token.value)
        assert abi.velnosub_abi_dialog_count(result.value) < 0
        original = workdir / "dialog-original.ass"
        assert abi.velnosub_abi_save(document, bytes(original)) == 0
        assert b"pending" not in original.read_bytes()

        assert start() == 1, abi.velnosub_abi_last_error()
        assert abi.velnosub_abi_dialog_set(result.value, 0, b"User input") == 0
        assert abi.velnosub_abi_macro_continue(token.value, 0, ctypes.byref(result)) == 1
        assert abi.velnosub_abi_macro_continue(token.value, 0, ctypes.byref(result)) == 0
        try:
            output = workdir / "dialog-result.ass"
            assert abi.velnosub_abi_save(result.value, bytes(output)) == 0
            assert b"User input" in output.read_bytes()
        finally:
            abi.velnosub_abi_close(result.value)
    finally:
        abi.velnosub_abi_close(document)


def test_resolution_resample(abi, workdir):
    source = workdir / "resample.ass"
    fixture = (SEED / "karaskel-dene.ass").read_text()
    fixture = fixture.replace("[Script Info]", "[Script Info]\nPlayResX: 640\nPlayResY: 360")
    fixture = fixture.replace(r"{\k30}ka", r"{\pos(100,50)\k30}ka", 1)
    source.write_text(fixture)
    original = abi.velnosub_abi_open(bytes(source))
    assert original >= 0, abi.velnosub_abi_last_error()
    try:
        draft = abi.velnosub_abi_snapshot(original)
        assert draft >= 0, abi.velnosub_abi_last_error()
        try:
            assert abi.velnosub_abi_resample(draft, 0, 720, 0) < 0
            assert abi.velnosub_abi_resample(draft, 1280, 720, 0) == 0, abi.velnosub_abi_last_error()
            output = workdir / "resampled.ass"
            assert abi.velnosub_abi_save(draft, bytes(output)) == 0
            rendered = output.read_text(encoding="utf-8-sig")
            assert "PlayResX: 1280" in rendered and "PlayResY: 720" in rendered
            assert r"\pos(200,100)\k30" in rendered
            assert "Style: Default,Arial,40," in rendered
        finally:
            abi.velnosub_abi_close(draft)
        original_output = workdir / "unchanged.ass"
        assert abi.velnosub_abi_save(original, bytes(original_output)) == 0
        assert r"\pos(100,50)\k30" in original_output.read_text(encoding="utf-8-sig")
    finally:
        abi.velnosub_abi_close(original)


def main():
    os.environ["VELNOSUB_AEGIINC"] = str(INCLUDES)
    os.environ["VELNOSUB_DATA_DIR"] = str(INCLUDES)
    abi = ctypes.CDLL("libvelnosub-abi.so")
    abi.velnosub_abi_last_error.restype = ctypes.c_char_p
    config = ROOT / "src/libresrc/default_config.json"
    assert abi.velnosub_abi_init(bytes(config)) == 0
    with tempfile.TemporaryDirectory() as directory:
        workdir = Path(directory)
        test_video(abi, workdir)
        test_audio_and_embedded_ass(abi, workdir)
        test_automation_and_fonts(abi, workdir)
        test_interactive_macro(abi, workdir)
        test_resolution_resample(abi, workdir)
    print("Mobile ABI regressions passed")


if __name__ == "__main__":
    main()
