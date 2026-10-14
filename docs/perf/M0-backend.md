# M0 renderer decision: measurements and outcome

| | |
|---|---|
| Protocol | `docs/specs/m0-foundations.md` §12, as amended by §17.13 (precedence §18 > §17 > body) |
| Role | Performance (WP6), measurement mode |
| Round 1 | 2026-10-14, 13:21Z–13:39Z: the full protocol, C1–C7 and C1d |
| Round 2 | 2026-10-14, 14:32Z–14:49Z: re-measurement of the configurations affected by the DX12 pin (fix-wave row 2.4), see §0 |
| Result | **Windows: wgpu, pinned to DX12.** The pin has landed. Plain `cargo xtask bench all --renderer wgpu` now reports `dx12`, passes G1 and G3, and paints 0 idle frames. **WSLg: wgpu under X11**, because wgpu on Wayland crashes the WSLg compositor. The proposed outcome is still **W (extended)**. It needs Architect ratification, see §6. |
| Open | Round 2 ran **on battery**, so G4 for the pinned C1 on mains power is still to be measured (§5.1). G2 and the cold start need a human. |

This report gives the numbers and the decision. G2 (visual check) and the cold start after a reboot
need a human, so they are listed as pending. The agent never claims they passed.

---

## 0. Round 2: what changed and what was re-measured

**The change.** `apps/visual-assembler/src/lib.rs` gained `pin_windows_wgpu_backend`, which `launch` calls.

- It is compiled only under `#[cfg(all(windows, feature = "renderer-wgpu"))]`.
- When `WGPU_BACKEND` is unset and the wgpu setup is `WgpuSetup::CreateNew`, it sets the instance backends to `Backends::DX12`. This is §6.3 item 1 of round 1.
- QA added `apps/visual-assembler/tests/renderer_backend.rs`. It is a test, so it is not part of the binary.
- The X11-under-WSL change (outcome W) has **not** landed, because it waits for ratification.

Which configurations the change affects:

| Config | Affected? | Evidence | Round 2 |
|---|---|---|---|
| C1 | **Yes.** wgpu's pick changes from Vulkan to DX12. | The bench line reports `dx12` with no `WGPU_BACKEND` set | Re-measured |
| C1d | **No change in path.** It is the same binary as C1, but the pin is skipped when `WGPU_BACKEND` is set. | Backend `dx12`, as in round 1 | Re-measured as a control in the same session. The C1 vs C1d rule compares the two, and the power state differs from round 1. |
| C2 | **No.** The glow-only build does not compile the pin. | The current tree's glow `.exe` was rebuilt and compared section by section with the measured copy. `.text`, `.data`, `.pdata` and `.reloc` are identical. The only 24 differing bytes are the link timestamps (COFF header and 3 debug-directory entries) and the PDB GUID. | Not re-measured |
| C3–C7 | **No.** The pin is Windows-only. | The current tree's x86_64 Linux wgpu and glow binaries were rebuilt in WSL and compared with the measured bench copies. After `strip --strip-all`, only the 20-byte GNU build-ID payload differs. `.text`, `.rodata`, `.data.rel.ro` and `.eh_frame` are identical. | Not re-measured. **No WSLg GUI launch in round 2.** |

**Power state.** AC power has been offline since 13:39:38Z (System log, Kernel-Power event 105). It was still
offline at every check in round 2 and during a 20-minute poll (§2). The battery-mode power overlay is
**Best power efficiency** (`961cc777-…`). That breaks the §12.1 mains-power condition. So:

- G1 and G3 are read from the round-2 data. Exit codes and idle frame counts are not expected to depend on clock speed. The battery condition is still stated next to every result.
- G4 is **not evaluable** from round 2. The round-2 startup numbers serve only to compare C1 with C1d and to size the battery effect. They never feed a gate.

---

## 1. Context

### 1.1 Repository

- Git: the repository has **no commit yet**. Every report therefore records `"sha": "nogit"` and `"dirty": false`,
  because all files are still untracked.
- Round 1 measured the working tree as handed over after QA was green on 2026-10-14.
- Round 2 measured the tree after the DX12 pin (`lib.rs` modified 13:56Z) and QA's green re-run.
- The bench binaries are rebuilt from the tree by `cargo xtask bench`. In round 2, `target/bench/wgpu/visual-assembler.exe` (15,099,392 B) was built at 14:12:40Z, after the `lib.rs` change.

### 1.2 Host (Windows, reference machine)

| Item | Value |
|---|---|
| Machine | Lenovo 83JY (laptop) |
| CPU | AMD Ryzen AI 7 350 with Radeon 860M: 8 cores, 16 logical CPUs |
| GPU | AMD Radeon 860M Graphics, integrated with shared memory. Windows driver 32.0.22076.3001 (2026-07-19). Vulkan reports `AMD proprietary driver 25.20.76.03 (LLPC)`; GL reports `3.3.0 Core Profile Context 25.20.76.03.260706` |
| Second display adapter | Parsec Virtual Display Adapter 0.45.0.0 (fragile point F12). No run picked it: every run reports the AMD adapter |
| RAM | 31.3 GiB |
| Display | Internal panel 2880×1800 at 200 % scale |
| OS | Windows 11 Pro 25H2, build 10.0.26200.9711 (unchanged in round 2) |
| Power, round 1 | AC online from 12:56:41Z to 13:39:38Z (System log, Kernel-Power event 105). AC power-mode overlay = **Best performance** (`ded574b5-…`); base scheme Balanced. Every round-1 run in §3 falls inside the AC window. |
| Power, round 2 | **Battery**, charge 59 % → 57 %. DC overlay = **Best power efficiency** (`961cc777-…`); base scheme Balanced. |
| Load before the runs | CPU load about 3–5 % in both rounds. Interactive apps were open but idle: browser, Claude desktop, OCCT GUI (monitoring only); Task Manager in round 1. The agent cannot verify that nobody touched the mouse or keyboard. |
| Toolchain | rustc 1.99.0 (b940084d7 2026-09-28, LLVM 23.1.1), cargo 1.99.0 (5f94df478 2026-08-27), MSVC host `x86_64-pc-windows-msvc` |

### 1.3 WSL / WSLg

All values were re-checked in the round-2 pre-flight (14:32Z) and are unchanged.

| Item | Value |
|---|---|
| WSL | 2.7.14.0 |
| Kernel | 6.18.33.2-microsoft-standard-WSL2 |
| WSLg | 1.0.73.2 (weston 9.0.0, build 9.0.0-213-g04d436c7; FreeRDP c4030980; MSRDC 1.2.7214) |
| Direct3D / DXCore | 1.611.1-81528511 / 10.0.26100.1-240331-1435.ge-release |
| Distro | Ubuntu 26.04.1 LTS. The VM sees 8 logical CPUs and 15 GiB of RAM. |
| Mesa | 26.0.8-1ubuntu0.3 (`libgl1-mesa-dri`, `libegl-mesa0`, `mesa-vulkan-drivers`). llvmpipe and lavapipe use LLVM 21.1.8. |
| GPU path actually used | **Software rendering only**: llvmpipe (GL) and lavapipe (Vulkan, shown as `llvmpipe`). `/dev/dxg`, `libd3d12.so` and `d3d12_dri.so` are present, but Mesa did not select d3d12. On Wayland, EGL tries zink (`MESA: error: ZINK: failed to choose pdev`). On X11 it reports `DRI3 error: Could not get DRI3 device`, and Xwayland logs `glamor: GBM Wayland interfaces not available, falling back to sw`. This is fragile point F1 happening for real; it was not investigated further (no package changes allowed, and WSLg work stopped after 3 crashes). |
| Toolchain | rustc 1.99.0, cargo 1.99.0; `CARGO_TARGET_DIR=$HOME/.cache/visualassembler/target` (ext4) |
| Session state | Round 1: the same weston process had been up since 08:48:53 local time. The first X11 client of the session triggered `rdp_rail_wake_handler` at 15:32:58 local. Round 2: the weston instance started after crash 3 (`wayland-0` inode 44, created 13:39:18Z) was still running. Times in the weston logs are CEST (UTC+2). |

---

## 2. Commands

All Windows commands ran in Git Bash at the repository root. All WSL commands ran as
`wsl.exe -d Ubuntu -e bash -lc 'cd "/mnt/c/Users/sapin/Documents/Code Info/VisualAssembler" && export CARGO_TARGET_DIR=$HOME/.cache/visualassembler/target && <cmd>'`.
Default run counts were used: startup 1 warm-up + 20 runs, idle 1 warm-up + 3 runs of 10 s, with a 3 s settle.

| Round | Config | Environment | Exact command | Notes |
|---|---|---|---|---|
| 1 | C1 | Windows; no `WGPU_*` variable set | `cargo xtask bench all --renderer wgpu` | Before the pin: wgpu picked Vulkan |
| 1 | C1d | Windows | `WGPU_BACKEND=dx12 cargo xtask bench all --renderer wgpu` | §17.13 |
| 1 | C2 | Windows | `cargo xtask bench all --renderer glow` | |
| 1 | C7 | WSLg X11 | `env -u WAYLAND_DISPLAY cargo xtask bench all --renderer glow` | 3 clean probes first |
| 1 | C6 | WSLg X11 | `env -u WAYLAND_DISPLAY cargo xtask bench all --renderer wgpu` | 3 clean probes first |
| 1 | C8 | WSLg X11 | not run | Only needed if C6 fails G1–G3 (§12.1), and C6 passed |
| 1 | C5 | WSLg Wayland (`WAYLAND_DISPLAY=wayland-0`) | `cargo xtask bench all --renderer glow` | 3 clean probes first |
| 1 | C4 | WSLg Wayland | probes only: `WGPU_BACKEND=gl <copy> --bench startup` | Probe 1 crashed the compositor, so no bench (§17.13 step 5) |
| 1 | C3 | WSLg Wayland | probes only: `<copy> --bench startup` | Probes 1 and 2 crashed the compositor. The session crash limit was reached, so no bench. |
| 2 | C1 | Windows; no `WGPU_*`, `CARGO_TARGET_DIR` or `RUSTFLAGS` variable set; battery | `cargo xtask bench all --renderer wgpu` | 14:32:57Z–14:34:13Z. With the pin: wgpu uses DX12 |
| 2 | C1d | Windows; battery | `WGPU_BACKEND=dx12 cargo xtask bench all --renderer wgpu` | 14:34:19Z–14:35:35Z. Control |

WSLg procedure (§17.13), round 1, as run:

- **Pre-flight** (once, 13:31Z):
  - recorded `wsl.exe --version`;
  - checked that `/mnt/wslg/runtime-dir/wayland-0` and `/tmp/.X11-unix/X0` exist;
  - copied the last 100 lines of `/mnt/wslg/weston.log` and `/mnt/wslg/stderr.log` to the session scratchpad (not the repo). The baseline had 0 crash markers.
- **Bench copies for the probes.** The runner's step 1 was reproduced:
  - `cargo build --release -p visual-assembler --no-default-features --features renderer-<r>`;
  - then `cp $CARGO_TARGET_DIR/release/visual-assembler $CARGO_TARGET_DIR/bench/<r>/visual-assembler`.
- **Each probe:** `timeout 75 env <config vars> $CARGO_TARGET_DIR/bench/<r>/visual-assembler --bench startup </dev/null`. The probe counts as a crash if any of these holds:
  - new `SIGSEGV`, `Segmentation fault`, `backtrace`, `signal 11` or `weston 9.0.0` lines appear in either log (checked 2 s after exit, from byte offsets taken before the launch);
  - the `wayland-0` socket disappears or its inode changes;
  - the GUI exits 3 with a connection error.
- **Recovery:** poll every 5 s for up to 180 s until `wayland-0` exists **and** `env -u WAYLAND_DISPLAY $CARGO_TARGET_DIR/bench/glow/visual-assembler --renderer glow --bench startup` exits 0.

WSLg procedure, round 2:

- **Pre-flight only** (14:32:37Z). It recorded `wsl.exe --version` and found both sockets present (`wayland-0` inode 44). It copied the last 100 lines of both logs to the scratchpad as a new baseline.
- The full logs still hold exactly the 3 round-1 crash markers and 4 weston start headers. **No compositor crash happened between the rounds.**
- No WSLg GUI was launched in round 2, because no WSLg configuration is affected (§0). The only WSL commands were builds and binary comparisons.

Extra commands (informational):

- Round 1, on AC: a Vulkan idle-frame check on Windows (`target/bench/wgpu/visual-assembler.exe --bench idle --duration-secs 3`, then the same with `--duration-secs 20`).
- Round 1, on AC: a module and memory listing on Windows (PowerShell `Get-Process`, its `.Modules` and memory fields), 7 s into `visual-assembler.exe --bench idle --duration-secs 12`. It ran three times: wgpu with `WGPU_BACKEND=dx12`, wgpu with `WGPU_BACKEND=vulkan`, and glow.
- Round 2, on battery: the same module listing for the pinned wgpu binary with no `WGPU_BACKEND`, then with `WGPU_BACKEND=vulkan`.
- Round 2, on battery: a windowless init-timing probe (§5.1). It was built **outside the repository**, in the session scratchpad, offline from the repository's `Cargo.lock` versions: egui 0.36.2, egui-wgpu 0.36.2 (default features, as `eframe/wgpu` enables them), wgpu 30.0.1, pollster 1.0.1. It was run 1 warm-up + 10 times per backend set.
- Round 2: an AC poll every ~32 s from 14:42Z to 15:02:42Z. It read `PowerOnline=False` every time (§0).
- E3 and E4: see §8.

---

## 3. Results

Units:

- times in ms;
- spawn→frame and startup_ms come from the startup scenario;
- idle CPU and idle frames come from the idle scenario;
- peak RSS and peak private are the startup-scenario p50 (idle p50 in brackets);
- binary size is `binary_bytes` of the measured copy.

### 3.1 Round 1 (mains power, Best performance)

| # | Renderer | Backend | Adapter | Window | spawn→frame p50/p95 | startup_ms p50/p95 | idle CPU % p50/p95 | idle frames p50/max | peak RSS p50 MiB | peak private p50 MiB | binary |
|---|---|---|---|---|---|---|---|---|---|---|---|
| C1 (before the pin) | wgpu | vulkan | AMD Radeon (TM) 860M Graphics | win32 | 435.7 / **530.5** | 422.5 / 516.1 | 0.00 / 0.00 | **3 / 3** | 382.3 (387.5) | 375.9 (383.5) | 15,101,440 B (14.40 MiB) |
| C1d | wgpu | dx12 | AMD Radeon(TM) 860M Graphics | win32 | 435.5 / 464.8 | 422.7 / 451.5 | 0.00 / 0.00 | 0 / 0 | 342.4 (346.4) | 299.0 (302.9) | 15,101,440 B (14.40 MiB) |
| C2 | glow | gl (WGL) | AMD Radeon (TM) 860M Graphics | win32 | 278.6 / 301.8 | 267.4 / 291.0 | 0.00 / 0.00 | 0 / 0 | 92.7 (113.5) | 117.8 (117.9) | 7,968,256 B (7.60 MiB) |
| C3 | wgpu | — (no JSON line) | — | — | not benched: compositor crash 2/2 | — | — | — | — | n/a | 25,692,232 B (24.50 MiB) |
| C4 | wgpu | gl (probes 2–3) | llvmpipe (LLVM 21.1.8, 256 bits) | wayland (probes 2–3) | not benched: compositor crash 1/3 | probes 2–3: 217.6, 230.7 | — | — | probes: 151.1 | n/a | 25,692,232 B (24.50 MiB) |
| C5 | glow | gl (EGL) | llvmpipe (LLVM 21.1.8, 256 bits) | wayland | 79.3 / 86.0 | 77.3 / 83.9 | 0.01 / 0.02 | 0 / 0 | 123.8 (178.9) | n/a | 18,371,976 B (17.52 MiB) |
| C6 | wgpu | vulkan (lavapipe) | llvmpipe (LLVM 21.1.8, 256 bits) | xlib | 136.6 / 151.8 | 134.8 / 149.5 | 0.02 / 0.02 | 0 / 0 | 135.7 (144.1) | n/a | 25,692,232 B (24.50 MiB) |
| C7 | glow | gl (GLX/EGL) | llvmpipe (LLVM 21.1.8, 256 bits) | xlib | 97.3 / 107.8 | 95.5 / 106.0 | 0.02 / 0.02 | 0 / 0 | 118.7 (118.7) | n/a | 18,371,976 B (17.52 MiB) |

C2–C7 stay valid after the pin: their code is unchanged (§0).

### 3.2 Round 2 (battery, Best power efficiency; §12.1 power condition not met)

| # | Renderer | Backend | Adapter | Window | spawn→frame p50/p95 | startup_ms p50/p95 | idle CPU % p50/p95 | idle frames p50/max | peak RSS p50 MiB | peak private p50 MiB | binary |
|---|---|---|---|---|---|---|---|---|---|---|---|
| **C1 (pinned)** | wgpu | **dx12** | AMD Radeon(TM) 860M Graphics | win32 | 586.8 / 617.2 | 563.4 / 598.7 | 0.00 / 0.16 | **0 / 0** | 340.5 (344.6) | 299.1 (303.0) | 15,099,392 B (14.40 MiB) |
| C1d (control) | wgpu | dx12 | AMD Radeon(TM) 860M Graphics | win32 | 586.6 / 616.1 | 568.3 / 596.1 | 0.00 / 0.16 | 0 / 0 | 340.4 (344.3) | 299.0 (302.9) | 15,099,392 B (14.40 MiB) |

- **C1 now matches C1d.** p50 differs by +0.2 ms and p95 by +1.1 ms (0.2 %). RSS and private bytes match to 0.1 MiB, and both report the same adapter and driver string.
- **Idle CPU p95 of 0.16 %** is one Windows CPU-time tick (15.625 ms over 10 s) in one of the 3 idle runs, in both configurations. The other runs read 0.0 ms. This is far below G3's 1.0 %.
- **Memory, C1 before vs after the pin.** Memory does not depend on the power state: C1d's private bytes are 299.0 MiB on AC and on battery. Against round-1 C1 (Vulkan), the pinned C1 has 41.8 MiB less peak RSS (340.5 vs 382.3) and 76.8 MiB less peak private (299.1 vs 375.9).
- **The battery cost.** C1d went from 435.5 / 464.8 ms on AC (round 1) to 586.6 / 616.1 ms on battery: +35 % at p50 and +33 % at p95. CPU time at the first frame went from 273 ms to 437.5 ms (p50).
- **Other battery runs** (all excluded from the gates) agree, with p95 between 597 and 627 ms whether the pin or the variable selects DX12:
  - round 1's excluded C1d replication: 557.6 / 600.0 ms;
  - QA's runs after the pin, 14:05Z–14:12Z (§9.1): plain 574.6 / 626.9 and 575.6 / 597.9; `WGPU_BACKEND=dx12` 574.0 / 597.0.

Driver strings:

| Config | Driver |
|---|---|
| C1 round 1 (Vulkan) | `AMD proprietary driver 25.20.76.03 (LLPC)` |
| C1 round 2 (pinned), C1d both rounds | `32.0.22076.3001` |
| C2 | `3.3.0 Core Profile Context 25.20.76.03.260706` |
| C5, C7, C4 probes | `4.5 (Core Profile) Mesa 26.0.8-1ubuntu0.3` |
| C6 | `llvmpipe Mesa 26.0.8-1ubuntu0.3 (LLVM 21.1.8)` |

The window system matched §12.1 in every case:

- `wayland` for C5 and for C4 probes 2–3;
- `xlib` for C6 and C7;
- `win32` for C1, C1d and C2;
- C3 never printed a line.

### 3.3 Observations

**C1 idle frames, round 1 (Vulkan).** With Vulkan, the idle window shows a fixed burst of frames, not a periodic repaint:

| Run | Window | Frames painted |
|---|---|---|
| Bench, 3 runs | 10 s | 3 each |
| Extra probe | 3 s | 2 |
| Extra probe | 20 s | 2 |
| Module-listing run | 12 s | 2 |

- The process CPU stays at 0.0 ms, below one Windows CPU-time tick.
- The frames therefore fall roughly 3–6 s after the first frame, after the settle. This is a post-map transient on the Vulkan present path. WP2 had seen one late Vulkan frame about 2 s after the first frame.
- DX12 (C1d) and glow (C2) paint 0 frames.
- By the letter of G3 (median `frames_painted == 0`), round-1 C1 failed.

**C1 idle frames, round 2 (pinned).** 0 frames in all 3 idle bench runs and in the 12 s module-listing run. With `WGPU_BACKEND=vulkan`, the round-2 binary still paints 2 frames in 12 s. The Vulkan symptom is unchanged, but Vulkan is now opt-in only.

**What the pin removes from the process** (module listing 7 s into an idle run, round 2):

| Process | Modules | Mapped images | `vulkan-1.dll` | `amdvlk64.dll` (Vulkan ICD) | `atio6axx.dll` (GL ICD) | DX12 images |
|---|---|---|---|---|---|---|
| Pinned, no `WGPU_BACKEND` | 65 | 175.4 MiB | absent | absent | absent | `D3D12Core.dll`, `amdxc64.dll`, `d3dcompiler_47.dll`, `d3d10warp.dll` |
| `WGPU_BACKEND=vulkan` | 65 | 268.0 MiB | loaded | loaded (99.9 MiB) | absent | `D3D12Core.dll`, `amdxc64.dll` |

The pinned process has the same image set as round 1's `WGPU_BACKEND=dx12` listing (175.4 MiB). No Vulkan or GL driver is loaded.

**C1 p95, round 1.** With nearest rank on n = 20, the p95 is the 2nd-largest sample: run 1 at 530.5 ms. Run 10, at 781.8 ms, is the maximum. The other 18 runs are 426–452 ms.

**Warm-up runs** (first launch of each bench):

| Round | Platform | C1 | C1d | C2 | C7 | C6 | C5 |
|---|---|---|---|---|---|---|---|
| 1 (AC, after a fresh build: Defender scan and driver caches) | Windows | 1064.4 | 679.0 | 528.6 | — | — | — |
| 1 | WSLg | — | — | — | 112.1 | 131.3 | 82.4 |
| 2 (battery, bench copy already built) | Windows | 1365.4 | 1056.1 | — | — | — | — |

These are not cold starts (see §7).

**Excluded run (round 1).** A replication of C1d at 13:45:56Z is excluded from every table and gate, because it ran on
**battery**: AC went offline at 13:39:38Z, which breaks the §12.1 power condition. It also followed a fresh rebuild of the
bench copy. Its numbers:

| Metric | Value |
|---|---|
| spawn→frame p50 / p95 | 557.6 / 600.0 ms |
| idle-run startup_ms p50 | 1050.3 ms |
| idle frames | 0 |
| peak RSS p50 | 343.0 MiB |
| report | `bench/results/20261014T134749Z-nogit.json` |

**Bench wall time.** In round 1, C1 (Vulkan) took 2 min 39 s wall and C1d took 1 min 8 s, with the same run counts.
In round 2, the pinned C1 and C1d each took 1 min 16 s. The long round-1 wall time was therefore specific to the Vulkan path.
Process teardown is outside the protocol, so it was not investigated further.

### 3.4 Peak RSS: wgpu more than 100 MB above glow on Windows (§12.3 requires an explanation)

The gaps over C2's 92.7 MiB are:

- +289.6 MiB for round-1 C1 (Vulkan);
- +249.7 MiB for C1d;
- +247.8 MiB for the pinned C1 (round 2; memory does not depend on power, see §3.2).

The explanation is consistent with the data but **was not verified by a code change**.

1. **The GPU memory allocator's default block sizes on a shared-memory GPU.**
   - eframe/egui-wgpu 0.36.2 build the device with `..Default::default()`, so `MemoryHints::Performance` applies.
   - In wgpu-hal 30.0.1, that hint gives gpu-allocator blocks of at least 128 MiB for device memory and 64 MiB for host memory.
   - On the integrated Radeon 860M, those heaps are system memory charged to the process: about 192 MiB.
   - The measured private-bytes gap is 299.0 − 117.8 = 181.2 MiB (C1d vs C2), which matches. The pinned C1 gives the same figure (299.1).
2. **Driver images mapped in the working set** (sizes from the module listings):

   | Process | Driver images |
   |---|---|
   | DX12 (C1d, pinned C1) | `amdxc64.dll` (75.2 MiB, AMD shader compiler), `D3D12Core.dll`, `d3dcompiler_47.dll` (4.5 MiB), `d3d10warp.dll` (5.7 MiB) |
   | Vulkan | `amdvlk64.dll` (99.9 MiB) plus `amdxc64.dll` (75.2 MiB), the highest of the three |
   | glow | `atio6axx.dll` (62.8 MiB) |

   Total mapped images: DX12 175.4 MiB, Vulkan 268.0 MiB, glow 138.5 MiB.

M7 candidate (not measured here): a custom `device_descriptor` with `memory_hints: MemoryHints::MemoryUsage`
(8 MiB device / 4 MiB host blocks), then re-measure. Under §12.3, RSS does not decide alone.

---

## 4. G2 checklist (manual, one launch per configuration)

The check (§12.2): the window appears with a readable title and text; it can be moved, resized, minimized, restored and closed; there is no flicker or black frame after a resize.
Command: `cargo run --release -p visual-assembler --no-default-features --features renderer-<r>`, with the configuration's
environment. In WSL, export the `CARGO_TARGET_DIR` above first.

| Config | Command environment | Status | Observations |
|---|---|---|---|
| C1 | none | **pending human check** | Since the pin, this is the DX12 path, the same path as C1d. Automated runs: 25/25 in round 1 (Vulkan) and 25/25 in round 2 (DX12), plus 1 module-listing launch, all opened and closed the window. |
| C1d | `WGPU_BACKEND=dx12` | **pending human check** | 25/25 automated runs were fine in each round |
| C2 | none | **pending human check** | 25/25 automated runs were fine |
| C3 | none (Wayland) | **pending human check, not recommended** | 2 of 2 launches crashed weston, which closes every WSLg window. G1 already fails. If a human still wants G2, run it last and expect the compositor to restart. |
| C4 | `WGPU_BACKEND=gl` (Wayland) | **pending human check, not recommended** | 1 of 3 launches crashed weston (same warning as C3) |
| C5 | none (Wayland) | **pending human check** | Also decides the §12.4 CSD conditional: is the Wayland window decorated and movable? |
| C6 | `env -u WAYLAND_DISPLAY` | **pending human check** | This is the WSLg path proposed in §6 |
| C7 | `env -u WAYLAND_DISPLAY` | **pending human check** | |

---

## 5. Gate table

G1 counts warm-ups plus measured runs, startup and idle: 21 + 4 = 25 per bench. G4 is a target, not a blocking gate (§17.13); for WSLg it is reported only.
The C1 column is the pinned build (round 2). Round-1 C1 (Vulkan, before the pin) had G1 pass 25/25, G3 **fail** (median frames 3) and G4 **missed** (530.5).

| Gate | C1 (pinned) | C1d | C2 | C3 | C4 | C5 | C6 | C7 |
|---|---|---|---|---|---|---|---|---|
| G1 reliability | pass (25/25, round 2) | pass (25/25 in each round) | pass (25/25) | **fail**: compositor crash 2/2 probes | **fail**: compositor crash 1/3 probes | pass (3/3 probes, 25/25) | pass (3/3 probes, 25/25) | pass (3/3 probes, 25/25) |
| G2 visual | pending human | pending human | pending human | pending (not recommended) | pending (not recommended) | pending human | pending human | pending human |
| G3 idle | pass: 0 frames, CPU 0.00 % (round 2, battery) | pass: 0 frames, 0.00 % (both rounds) | pass: 0 frames, 0.00 % | not measured | not measured | pass: 0 frames, 0.01 % | pass: 0 frames, 0.02 % | pass: 0 frames, 0.02 % |
| G4 startup, p95 ≤ 500 ms (target) | **pending: needs a run on AC.** On battery: 617.2, and C1d's control is 616.1 | met on AC (464.8, round 1). On battery: 616.1 | met (301.8) | reported: n/a | reported: n/a | reported: 86.0 | reported: 151.8 | reported: 107.8 |

C8 was not run, because C6 passed G1–G3.

### 5.1 G4 analysis

**Status.** On AC in Best performance mode, round-1 C1d met the target with a margin of only 35 ms (7 %). The pinned C1
now runs the same path as C1d: on battery, the two are within 0.2 % of each other. That is a strong hint that the
pinned C1 will match C1d on AC, **but no AC number exists for it yet**. The row 2.4 acceptance check ("p95 ≤ 500 ms on
AC") stays open until that run (§6.3).

On battery in Best power efficiency mode, no DX12 run meets the target. Six 20-run battery benches have p95 between 597 and
627 ms (§3.2). The gap is +117 ms (+23 %) for the pinned C1. So the target holds only on AC in Best performance mode.

**Where the time goes, process level** (same method as round 1):

- **Before `main()`.** spawn→frame minus startup_ms has a median of 19.9 ms (pinned C1) and 18.3 ms (C1d) on battery. On AC, round 1 had 12.75 ms (C1d), 13.55 ms (C1) and 10.9 ms (C2) on Windows, and 1.7–1.9 ms on Linux. Process creation and DLL loading are therefore not the issue: 96–97 % of the time is inside the process, from `main()` to the first frame.
- **Inside the process, on AC (round 1):** glow takes 267 ms; wgpu takes 423 ms on DX12 and 422 ms on Vulkan.
- **CPU-bound.** CPU time at the first frame is 273 ms on AC (C1d, about 65 % of wall time). On battery it is 414 ms (pinned C1) and 437.5 ms (C1d), 73–77 % of wall time. The startup path is mostly CPU work (the main thread plus driver threads), so it scales with the clock speed the power mode allows.

**Where the time goes inside wgpu's init (round 2, new).** The windowless probe (§2) times, in a fresh process, the steps
egui-wgpu's `RenderState::create` runs before the first frame:

- instance;
- adapter enumeration;
- adapter request;
- device request;
- `egui_wgpu::Renderer::new` (shader module and render pipeline, which on DX12 goes naga to HLSL to FXC);
- then one renderer-independent egui frame of the M0 UI (layout and tessellation with the font atlas).

Battery, Best power efficiency; median [min–max] of 10 runs after 1 warm-up; ms:

| Backend set | Instance | Enumerate adapters | Request adapter | Request device | `Renderer::new` | egui frame | Total from `main` |
|---|---|---|---|---|---|---|---|
| **DX12 only (the pin)** | 24.8 [22.4–27.9] | **270.8** [252.8–316.5] (3 adapters) | 2.2 | 79.7 [74.4–84.2] | 18.1 [16.9–20.0] | 2.2 | **394.4** [379.8–440.9] |
| DX12 only, egui's explicit enumeration skipped | 24.4 | — | **275.5** [266.9–293.5] | 76.2 | 18.8 | 2.2 | 398.8 |
| Vulkan only | 167.2 | 6.9 (1) | 6.3 | 100.2 | 3.3 | 4.2 | 292.8 |
| GL only | 253.9 | 0.5 (1) | 0.1 | 145.8 | 8.4 | 2.8 | 411.4 |
| PRIMARY \| GL (egui's default before the pin; picks Vulkan) | 217.1 | 293.8 (5) | 6.7 | 61.6 | 2.3 | 2.5 | 567.9 |

The DX12-only row is the second batch. The first batch ran before the other sets and gave a median total of
404.6 ms; its first 3 runs were slower (536–828 ms).

What this shows:

1. **DX12 adapter discovery is the largest single cost:** about 270 ms, 69 % of the windowless DX12 init.
   - wgpu finds 3 DX12 adapters: `AMD Radeon(TM) 860M Graphics` listed twice, and `Microsoft Basic Render Driver` (WARP, type Cpu).
   - The cost is discovery itself, not egui's extra `enumerate_adapters` call: when that call is skipped, `request_adapter` takes the same 275 ms.
   - Suspected content (not profiled): opening a D3D12 device on each adapter to test it, which loads the AMD user-mode driver (`amdxc64.dll`, 75 MiB) and WARP (`d3d10warp.dll`).
2. **Device creation** is about 80 ms (20 %).
3. **Shader translation and pipeline creation are small.** `Renderer::new` takes 18 ms (5 %). Round 1's suspicion about FXC is therefore minor: DXC or a pipeline cache can save at most about 18 ms.
4. **The renderer-independent egui frame is about 2 ms.**
5. **The windowless DX12 init (about 394 ms) is about 70 % of the pinned C1's startup_ms p50 (563.4 ms)** in the same power state. The remaining ~170 ms are not separated. They cover window creation, surface and swapchain setup, UI Automation (AccessKit), the font-atlas upload and the first present.
6. **The pin removes Vulkan and GL instance creation** from plain C1 (PRIMARY | GL: 217 ms instance, 5 adapters).
   - Caution: round 1 on AC had the same startup_ms p50 for C1 (Vulkan) and C1d. The windowed paths after init must therefore differ between Vulkan and DX12.
   - So these windowless numbers do **not** predict the pinned C1's AC p95. They only split wgpu's init cost.

**Inputs for M7,** ordered by the measured share (battery figures):

1. DX12 adapter discovery, about 270 ms: can the WARP adapter and the duplicate AMD entry be skipped? One route is `WgpuSetup::Existing` with an adapter chosen up front. Re-measure afterwards.
2. The part outside wgpu's init, about 170 ms: window, surface and swapchain, AccessKit, first present. Split it with WPR/ETW, which needs admin rights.
3. Device creation, about 80 ms, with `MemoryHints::MemoryUsage` (§3.4) as a candidate that may also lower RSS.
4. DXC or a pipeline cache: at most about 18 ms.

---

## 6. Decision

### 6.1 Windows (§17.13, C1 vs C1d rule)

Round 1:

- C1 (wgpu's default pick here was **Vulkan**) failed G3: idle frames 3.
- C1d (DX12) passed G1 and G3, and met G4. G2 is pending.
- C1d was the only one of the two that passed G1–G3, and it also had the lower p95: 464.8 ms vs 530.5 ms, 12.4 % lower.
- **dx12 won.** wgpu's default pick was not dx12, so WP2 had to pin `Backends::DX12` on Windows when `WGPU_BACKEND` is unset (§17.13; fix-wave row 2.4).

Round 2 confirms the pin:

- Plain C1 now reports `dx12`, passes G1 (25/25) and G3 (0 frames), and is within 0.2 % of C1d (p95 617.2 vs 616.1 ms on battery).
- The rule gives **dx12** whichever way it is read: both configurations pass G1 and G3 (G2 pending for both), and the difference is under 10 %.
- **Outcome D is not triggered:** both C1 and C1d pass G1 and G3. G2 is still pending.

### 6.2 WSLg (§12.3, §12.4, §17.13 item 8)

Unchanged by round 2: the pin does not touch Linux (§0), and no WSLg run was repeated.

- **X11:** wgpu with its default backend passes G1–G3 (C6), and so does glow (C7). C8 was not needed.
- **Wayland:** glow passes G1–G3 (C5). wgpu fails G1 with its default backend (C3: compositor crash, 2/2) **and** with `WGPU_BACKEND=gl` (C4: compositor crash, 1/3).
- §12.3's wgpu condition ("for each of Wayland and X11, at least one wgpu configuration passes G1–G3") **is not met for Wayland**.

How each outcome matches, taken literally:

| Outcome | Condition | Matches? |
|---|---|---|
| A | wgpu passes everywhere with its default backend | No: C3 fails |
| B | wgpu passes with `WGPU_BACKEND=gl` | No: C4 failed as well |
| C | wgpu fails in WSLg on every backend, and glow passes | Only for Wayland. wgpu passes on X11 (C6). Also, C's runtime fallback reacts to a `run_native` error, which only happens **after** weston has already died and taken every other WSLg window with it. So it does not prevent the failure. |
| W | C3, C4 **and** C5 all fail G1 | No: C5 passes. Here only the wgpu paths fail on Wayland. |

**Proposed outcome: W (extended to a wgpu-only Wayland failure).** It needs Architect/orchestrator ratification, because the literal condition of W is not met.

Rationale:

1. W's remedy (X11 under WSL) is the only remedy in §12.4/§17.13 that avoids the failure mode, which is a compositor SIGSEGV.
2. X11 under WSLg passes G1–G3 with wgpu (C6). It had **0 crashes in 59 X11 launches** (C6: 28, C7: 28, recovery probes: 3).
3. It keeps a single default renderer (wgpu), in line with §12.3's Windows ARM64 rationale.
4. Under W, "the WSLg renderer decision uses the X11 results only" (§17.13 item 8).

Native Linux Wayland was not measured and keeps winit's default.

**Chosen renderer: wgpu.** `default = ["renderer-wgpu"]` stays. `renderer-glow` stays a fallback that CI compiles.
glow is faster to start (C2 p95 301.8 ms) and lighter (92.7 MiB RSS). Under §12.3 those numbers are reported but do
not decide alone. With the DX12 pin, wgpu passes G1 and G3 on Windows. It met G4 on AC as C1d, and the pinned C1 still needs its own AC run. G2 is pending.

### 6.3 Follow-up commits (WP2, fix-wave row 2.4), then a QA re-run

1. **DX12 pin: landed** (`pin_windows_wgpu_backend` in `apps/visual-assembler/src/lib.rs`; QA guard tests in `apps/visual-assembler/tests/renderer_backend.rs`).
   - Acceptance: the plain `cargo xtask bench all --renderer wgpu` must report backend `dx12`, idle frames 0 and p95 ≤ 500 ms on AC.
     - backend `dx12`: **met** (round 2 and QA's 3 plain runs);
     - idle frames 0: **met**;
     - p95 ≤ 500 ms on AC: **open**, because only battery data exists. On battery, 617.2 ms; C1d's control is 616.1 ms.
   - Still to do: one AC run in Best performance mode, recorded here. No code change is expected from it, because G4 does not block (§17.13).
   - The app README is WP7's scope. `apps/visual-assembler/README.md` does not exist yet.
2. **X11 under WSL: not landed** (outcome W, waiting for ratification).
   - Add a Linux-only dependency `winit = { version = "=0.30.13", default-features = false, features = ["x11"] }`: Apache-2.0, already in the graph, needs DEPENDANCE/IMPORT entries.
   - In `launch`, under `#[cfg(target_os = "linux")]`, when `bench::platform::is_wsl()` is true and env `VA_WAYLAND` is not `1`, set `options.event_loop_builder = Some(Box::new(|b| { winit::platform::x11::EventLoopBuilderExtX11::with_x11(b); }))`.
   - The README documents the WSLg limitation (weston 9.0.0 / WSLg 1.0.73.2 SIGSEGV with wgpu Wayland clients) and the `VA_WAYLAND=1` override.
   - Re-measure in a fresh session under §17.13: the plain WSL `cargo xtask bench all --renderer wgpu` must report `xlib`. From then on, measuring C3–C5 needs `VA_WAYLAND=1`.
3. **Not needed:** the WSL GL pin (outcome B does not apply). The CSD winit feature stays conditional on the C5 G2 check (§12.4).

---

## 7. Cold start

**Pending manual.** It needs a reboot, which the agent must never do. Right after a reboot, on AC in Best performance mode, run on Windows:
`cargo xtask bench startup --runs 1 --warmup 0 --renderer all`
Report it here, apart from the warm results. With the pin landed, the plain command already measures DX12, so the
`WGPU_BACKEND=dx12` variant is no longer needed. The warm-up runs in §3.3 are **not** cold starts:

- round 1, first launch after a build: 1064 / 679 / 529 ms;
- round 2: 1365 / 1056 ms on battery.

---

## 8. Toolchain evidence

### E3: aarch64 Linux cross-build (WSL)

Round 1 (2026-10-14, before the pin):

```
$ cargo build --release --locked --target aarch64-unknown-linux-gnu -p visual-assembler -p va
    Finished `release` profile [optimized] target(s) in 1.60s          # existing target dir, exit 0
$ CARGO_TARGET_DIR=~/.cache/visualassembler/perf-e3-clean (empty) cargo build --release --locked --target aarch64-unknown-linux-gnu -p visual-assembler -p va
   Compiling eframe v0.36.2
   Compiling visual-assembler v0.1.0 (/mnt/c/Users/sapin/Documents/Code Info/VisualAssembler/apps/visual-assembler)
    Finished `release` profile [optimized] target(s) in 1m 54s         # from scratch, exit 0
visual-assembler: ELF 64-bit LSB pie executable, ARM aarch64, version 1 (SYSV), dynamically linked, interpreter /lib/ld-linux-aarch64.so.1, BuildID[sha1]=210526b6fb574db2f414145ca7477f9b77def987, for GNU/Linux 3.7.0, not stripped
va:               ELF 64-bit LSB pie executable, ARM aarch64, version 1 (SYSV), dynamically linked, interpreter /lib/ld-linux-aarch64.so.1, BuildID[sha1]=95419fab413bdbbe8e74d02455a438626d7e4008, for GNU/Linux 3.7.0, not stripped
visual-assembler NEEDED: libgcc_s.so.1, libm.so.6, libc.so.6
```

Round 2 (2026-10-14T14:38:05Z, after the pin; existing target dir):

```
$ cargo build --release --locked --target aarch64-unknown-linux-gnu -p visual-assembler -p va
    Finished `release` profile [optimized] target(s) in 3.27s          # exit 0; visual-assembler last linked 14:12:41Z, after the lib.rs change
visual-assembler: ELF 64-bit LSB pie executable, ARM aarch64, version 1 (SYSV), dynamically linked, interpreter /lib/ld-linux-aarch64.so.1, BuildID[sha1]=210526b6fb574db2f414145ca7477f9b77def987, for GNU/Linux 3.7.0, not stripped
va:               ELF 64-bit LSB pie executable, ARM aarch64, version 1 (SYSV), dynamically linked, interpreter /lib/ld-linux-aarch64.so.1, BuildID[sha1]=95419fab413bdbbe8e74d02455a438626d7e4008, for GNU/Linux 3.7.0, not stripped
visual-assembler NEEDED: libgcc_s.so.1 libm.so.6 libc.so.6
va NEEDED: libgcc_s.so.1 libc.so.6
```

- The round-2 aarch64 `visual-assembler` keeps round 1's BuildID. This is consistent with a Windows-only change.
- Tools: target `aarch64-unknown-linux-gnu` installed; linker `aarch64-linux-gnu-gcc` (Ubuntu 15.2.0-16ubuntu1) 15.2.0.
- Tier: **compiled only**. The binary was not run.

### E4: Windows static CRT

Round 2 (2026-10-14T14:37:52Z, after the pin): `cargo build --release --locked -p visual-assembler -p va` (exit 0), then
`dumpbin /nologo /dependents` from MSVC 14.44.35207 Hostx64/x64:

```
target/release/visual-assembler.exe (15099392 bytes; default features = renderer-wgpu)
    oleaut32.dll kernel32.dll user32.dll api-ms-win-core-synch-l1-2-0.dll ntdll.dll psapi.dll
    combase.dll ole32.dll shell32.dll uiautomationcore.dll advapi32.dll gdi32.dll opengl32.dll
    dxgi.dll setupapi.dll bcryptprimitives.dll KERNEL32.dll USER32.dll imm32.dll dwmapi.dll uxtheme.dll
target/release/va.exe (743424 bytes)
    kernel32.dll api-ms-win-core-synch-l1-2-0.dll ntdll.dll KERNEL32.dll
target/bench/wgpu/visual-assembler.exe (15099392 bytes): same list as target/release/visual-assembler.exe
target/bench/glow/visual-assembler.exe (7968256 bytes)
    kernel32.dll oleaut32.dll uiautomationcore.dll user32.dll gdi32.dll ntdll.dll combase.dll ole32.dll
    api-ms-win-core-synch-l1-2-0.dll psapi.dll shell32.dll bcryptprimitives.dll OPENGL32.dll KERNEL32.dll
    USER32.dll GDI32.dll ADVAPI32.dll imm32.dll dwmapi.dll uxtheme.dll
forbidden-import check (VCRUNTIME140 / api-ms-win-crt-*): 0 matches in all four images
```

- The import lists are identical to round 1's. Only the wgpu image size changed: 15,101,440 → 15,099,392 B.
- Every output was checked to contain dumpbin's `Image has the following dependencies` header. This guards against a false 0. In Git Bash, `/nologo` must be passed with `MSYS_NO_PATHCONV=1`, or dumpbin receives a mangled path and prints no dependency list at all.

---

## 9. Raw data (informational; not committed)

### 9.1 Bench reports

Path: `bench/results/`, gitignored. All have `nogit`, `dirty: false`.

| File | Config | Note |
|---|---|---|
| `20261014T132405Z-nogit.json` | C1 round 1 (Vulkan) | Windows, AC |
| `20261014T132553Z-nogit.json` | C1d round 1 | Windows, AC |
| `20261014T132756Z-nogit.json` | C2 | Windows, AC |
| `20261014T133424Z-nogit.json` | C7 | WSLg X11 |
| `20261014T133543Z-nogit.json` | C6 | WSLg X11 |
| `20261014T133709Z-nogit.json` | C5 | WSLg Wayland |
| `20261014T134749Z-nogit.json` | C1d replication | **Excluded**: on battery (§3.3) |
| `20261014T140505Z-nogit.json` | QA, plain wgpu after the pin (`bench all`; separate target dir `target/wp2`) | Informational, battery: dx12, 574.6 / 626.9 ms, idle frames 0 |
| `20261014T140535Z-nogit.json` | QA, `WGPU_BACKEND=dx12` (startup only) | Informational, battery: dx12, 574.0 / 597.0 ms |
| `20261014T140553Z-nogit.json` | QA, plain wgpu (startup only) | Informational, battery: dx12, 575.6 / 597.9 ms |
| `20261014T141244Z-nogit.json` | QA, plain wgpu (startup, 2 runs) | Informational, battery: dx12 |
| `20261014T143413Z-nogit.json` | **C1 round 2 (pinned)** | Windows, battery (§3.2) |
| `20261014T143535Z-nogit.json` | **C1d round 2 (control)** | Windows, battery (§3.2) |

The windowless probe's output and the binary comparisons of §0 are kept in the session scratchpad, not in the repo.

### 9.2 Probe counts (§17.13 step 5)

| Round | Config | Probes run | Clean | Compositor crashes (k) | Went to bench |
|---|---|---|---|---|---|
| 1 | C7 X11 glow | 3 | 3 | 0 | yes |
| 1 | C6 X11 wgpu | 3 | 3 | 0 | yes |
| 1 | C5 Wayland glow | 3 | 3 | 0 | yes |
| 1 | C4 Wayland wgpu-gl | 3 | 2 | **1/3** (probe 1) | no |
| 1 | C3 Wayland wgpu default | 2 | 0 | **2/2** (probes 1 and 2) | no (session limit reached) |
| 1 | Recovery probes (X11 glow) | 3 | 3 | 0 | — |
| 2 | none: no WSLg configuration was affected (§0) | 0 | — | 0 | — |

WSLg GUI launches in total:

- round 1: 14 probes + 75 bench runs + 3 recovery probes = 92;
- round 2: 0.

### 9.3 WSLg crash log summary

There were 3 compositor crashes, all in round 1, so WSLg work stopped (§17.13 step 6). The compositor recovered by
itself each time. **`wsl --shutdown` was not used** in either round. Nothing was rebooted. The round-2 pre-flight
found no new crash marker: `wayland-0` was still inode 44 and the logs still held 4 weston start headers.

| # | UTC | Trigger launch | stderr.log (WSLGd) | Restart | Recovery |
|---|---|---|---|---|---|
| 1 | 13:37:17 | C4 probe 1 (wgpu, `WGPU_BACKEND=gl`, Wayland) | `pid 13 terminated with signal 11, /usr/bin/sh -c /usr/bin/weston --backend=rdp-backend.so … --shell=rdprail-shell.so …` | new `weston 9.0.0` header 31 ms later; `wayland-0` inode 12→32; msrdc reconnected | X11 glow probe exit 0 at the first poll (13:37:49) |
| 2 | 13:38:33 | C3 probe 1 (wgpu default, Wayland) | `pid 26993 terminated with signal 11, …` | new header 12 ms later; inode 32→38 | first poll exit 0 (13:39:03) |
| 3 | 13:39:18 | C3 probe 2 (wgpu default, Wayland) | `pid 27420 terminated with signal 11, …` | new header 11 ms later; inode 38→44 | first poll exit 0 (13:39:29) |

What the GUI printed at each crash (exit code 3):

```
Io error: Broken pipe (os error 32)
Io error: Broken pipe (os error 32)
Io error: Broken pipe (os error 32)
error: cannot create the window or the renderer: winit EventLoopError: Exit Failure: 1
```

What the logs show:

- The old weston wrote no line for the crashing client before it died, and `weston.log` has no backtrace.
- The new instance's stderr shows `(EE) failed to read Wayland events: Broken pipe` (Xwayland).
- As in WP2's observation, every weston instance had logged `rdp_rail_wake_handler is called` plus `launching '/usr/libexec/weston-rdprail-shell'` on its first client. That was 4 min 19 s, 43 s and 14 s before crashes 1, 2 and 3.
- The full excerpts are kept in the session scratchpad, not in the repo.

Classification (§17.13 step 7):

- Every crash is a **G1 failure** of the configuration that triggered it (C4, C3).
- Attribution: a weston SIGSEGV is a **WSLg defect**.
- VisualAssembler is a **reproducible trigger** for C3 (k = 2 ≥ 2). It is not one for C4 (k = 1).
- The trigger is **renderer-specific, so it is a GPU-path trigger**:
  - glow on the same winit Wayland window path had 0 crashes in 28 launches;
  - wgpu on Wayland crashed 3 times in 5 launches;
  - the main suspect is the Vulkan (lavapipe) Wayland WSI that the default wgpu instance creates.
- Filing an issue on `microsoft/wslg` needs the user's consent (public post). It was not done.

### 9.4 CI GUI smoke

The JSON lines from the Windows x64/ARM64 runners are **not available yet**. The repository has not been pushed, so E2 is not observable yet.

---

## 10. Limitations

- **Display latency.** No physical display latency is measured. "First frame presented" is the §4.5 definition (F13).
- **Windows CPU-time granularity** is about 15.6 ms, i.e. about 0.16 % over 10 s. Every Windows idle CPU reading was 0.0 ms or exactly one tick.
- **WSLg results depend on the Mesa and WSLg versions.** Here, all WSLg rendering was **software** (llvmpipe/lavapipe, §1.3):
  - WSLg startup and RSS numbers do not represent a GPU path;
  - the weston SIGSEGV is specific to WSLg 1.0.73.2 / weston 9.0.0-213.
- **Round 2 ran on battery** in Best power efficiency mode, which breaks §12.1.
  - Its startup times are about a third higher than on AC and are compared only with each other.
  - The pinned C1 has no AC startup number yet.
  - Mains power stayed offline through a 20-minute poll, and the agent cannot plug the laptop in.
- **The windowless probe** is out-of-tree and approximate:
  - it reproduces the steps of egui-wgpu's `RenderState::create` without a window or surface;
  - it was built without the repository's static-CRT rustflags;
  - it ran on battery.
- **Thin G4 margin.** The AC margin of C1d is 7 %, from one unreplicated session.
- **No git sha:** the repository has no commit yet.
- **Not measured:** native Linux Wayland/X11 and Windows ARM64. ARM64 is compiled only, through E3 and CI.
- **Not attempted, by design of the procedure:** C3 and C4 benches, and C3's third probe.
- **G2 and cold start** are pending a human.
