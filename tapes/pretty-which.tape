# Drives a real, isolated Herdr server with VHS and fails if the popup does not
# render the KEYS view. Run from the repo root:
#   PRETTY_WHICH_VHS_ROOT=<scratch dir> vhs tapes/pretty-which.tape
# Writes screenshots and a recording of all three modes.
# Cross-reference: tapes/isolated-herdr.sh

Output "pretty-which-demo.gif"
Require herdr
Require cargo
Require jq

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
Screenshot "keys.png"
# VHS captures screenshots asynchronously; let it land before typing.
Sleep 2s

Ctrl+F
Type "next tab"
Wait+Screen@5s /Next tab  prefix\+n, alt\+right  config/
Sleep 1s

Ctrl+T
Wait+Screen@5s /Bindings · Tree/
Sleep 1s

Escape
Escape
Sleep 1s
# Default category navigation, then the actual prefix+w binding.
Ctrl+B
Space
Wait+Screen@10s /GROUPS/
Type "w"
Wait+Screen@5s /Workspaces · press/
Screenshot "workspace-group.png"
Sleep 1s
Type "w"
Wait+Screen@10s /Choose workspace/
Wait+Screen@5s /pwvhs|pretty-which/
Screenshot "workspace-picker.png"
Sleep 1s
Escape
Sleep 1s
# Launcher execution creates a real new tab; assert it by querying the server.
Ctrl+B
Type "/"
Wait+Screen@10s /Herdr launcher/
Type "new tab"
Wait+Screen@5s /New tab/
Screenshot "launcher.png"
Sleep 1s
Enter
Sleep 2s
Type "herdr tab list | jq -e '.result.tabs | length == 2' && printf VHS-TWO-%s TABS-OK" Enter
Wait+Screen@10s /VHS-TWO-TABS-OK/
# Shell runs through the same plugin binary and shows real output.
Ctrl+B
Type "!"
Wait+Screen@10s /Shell/
Type "printf VHS-SHELL-%s OK"
Enter
Wait+Screen@10s /VHS-SHELL-OK/
Screenshot "shell.png"
Sleep 1s
Escape
Sleep 1s
Escape
Sleep 1s
Hide
Type "herdr server stop" Enter
Sleep 2s
