# Drives a real, isolated Herdr server with VHS and fails if the popup does not
# render the KEYS view. Run from the repo root:
#   PRETTY_WHICH_VHS_ROOT=<scratch dir> vhs tapes/pretty-which.tape
# Writes assets/readme-cover.png, the README screenshot.
# Cross-reference: tapes/isolated-herdr.sh

Output "assets/pretty-which-demo.gif"
Require herdr
Require cargo

Set Shell "bash"
Set FontSize 15
Set Width 1800
Set Height 1000
Set Padding 16
Set TypingSpeed 40ms
Set Theme "Catppuccin Mocha"

Hide
Type "source tapes/isolated-herdr.sh" Enter
Wait@300s /^\$/
Type "herdr --session pwvhs" Enter
Sleep 4s
Type "echo VHS-UNDERLAY-MARKER" Enter
Sleep 1s
Show

# prefix+space is bound to the plugin action in the isolated config.
Ctrl+B
Space
Wait+Screen@20s /KEYS · ALL/
# The popup leaves the tiled pane visible around it (overlay placement would not).
Wait+Screen@5s /VHS-UNDERLAY-MARKER/
Wait+Screen@5s /prefix ctrl\+b/
Wait+Screen@5s /WORKSPACES/
Sleep 1s
Screenshot "assets/readme-cover.png"
# VHS captures screenshots asynchronously; let it land before typing.
Sleep 2s

Type "next tab"
Wait+Screen@5s /Next tab  prefix\+n, alt\+right  config/
Sleep 1s

Ctrl+T
Wait+Screen@5s /Bindings · Tree/
Sleep 1s

Escape
Sleep 1s
Hide
Type "herdr server stop" Enter
Sleep 2s
