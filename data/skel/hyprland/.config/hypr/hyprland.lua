-- Config minimal dari rakuos-bootstrap.
-- Hyprland 0.55+ memakai hyprland.lua; file .conf lama tidak dipakai.
-- Tanpa bar: desktop pure, keybind dasar supaya langsung bisa dipakai.
-- File ini milik user; aman diedit/dihapus.

hl.monitor({
    output   = "",
    mode     = "preferred",
    position = "auto",
    scale    = "auto",
})

local terminal    = "@TERMINAL@"
local launcher    = "fuzzel"
local browser     = "@BROWSER@"
local filemanager = "@FILEMANAGER@"
local mainMod     = "SUPER"

hl.on("hyprland.start", function()
    hl.exec_cmd("dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP")
end)

hl.config({
    general = {
        gaps_in     = 4,
        gaps_out    = 8,
        border_size = 2,
        layout      = "dwindle",
    },
    decoration = {
        rounding = 6,
    },
    input = {
        kb_layout    = "us",
        follow_mouse = 1,
        touchpad = {
            natural_scroll = true,
        },
    },
})

-- Ganti "fuzzel" ke perintah launcher shell (Noctalia/DMS) kalau perlu.
hl.bind(mainMod .. " + Return", hl.dsp.exec_cmd(terminal))
hl.bind(mainMod .. " + D",      hl.dsp.exec_cmd(launcher))
hl.bind(mainMod .. " + B",      hl.dsp.exec_cmd(browser))
hl.bind(mainMod .. " + E",      hl.dsp.exec_cmd(filemanager))

hl.bind(mainMod .. " + Q",         hl.dsp.window.close())
hl.bind(mainMod .. " + F",         hl.dsp.window.fullscreen({ action = "toggle" }))
hl.bind(mainMod .. " + SHIFT + E", hl.dsp.exit())

-- Fokus window.
hl.bind(mainMod .. " + left",  hl.dsp.focus({ direction = "left" }))
hl.bind(mainMod .. " + right", hl.dsp.focus({ direction = "right" }))
hl.bind(mainMod .. " + up",    hl.dsp.focus({ direction = "up" }))
hl.bind(mainMod .. " + down",  hl.dsp.focus({ direction = "down" }))

-- Pindahkan window.
hl.bind(mainMod .. " + SHIFT + left",  hl.dsp.window.move({ direction = "left" }))
hl.bind(mainMod .. " + SHIFT + right", hl.dsp.window.move({ direction = "right" }))
hl.bind(mainMod .. " + SHIFT + up",    hl.dsp.window.move({ direction = "up" }))
hl.bind(mainMod .. " + SHIFT + down",  hl.dsp.window.move({ direction = "down" }))

-- Mouse: drag dan resize window.
hl.bind(mainMod .. " + mouse:272", hl.dsp.window.drag(),   { mouse = true })
hl.bind(mainMod .. " + mouse:273", hl.dsp.window.resize(), { mouse = true })
