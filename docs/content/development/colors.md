---
title: "Colors"
date: 2021-09-20T10:53:55-07:00
weight: 999
summary: "List of colors used in ReMUD by name"
author: "Shaen"
tags: ["colors", "scripting"]
---

ReMUD supportes clients with true color, 256 color, 16 color, and no color. Colors will be downgraded to the set that a client supports using a best effort algorithm. For best results, use a true color capable client and, if necessary, signal to ReMUD that the client supports true color using `xterm-truecolor` as a terminal type during telnet negotiations. On \*nix, this can be accomplished by setting `TERM="xterm-truecolor"` when invoking telnet.

Colors rendered below assume the standard xterm color scheme. If the terminal background is white, or the color scheme is different, results will be different.

Colors are added to strings using color tags. Currently, only foreground text can be colorized. Here is a list of supported tags:

- `|#123456|`: start a hex color
- `|4|`: start a color using the specific xterm-256 color index
- `|Aquamarine1|`: start a named color (see the list below)
- `|-|`: end a color, resuming the previous color (if any)
- `||`: an escape to print a pipe

It is generally recommended to use named colors if possible: the degredation function from true to 256 color has a tendancy to select the closest gray instead of the closest color. This will provide the most consistent experience for clients with less-than true color support.

# Named Colors

Some color names have been modified from the original xterm colors to eliminate duplication.

## System Colors (16 color supported)

{{< color-grid grid-cols-8 >}}
{{< color Black "#000000" >}}
{{< color Maroon "#800000" >}}
{{< color Green "#008000" >}}
{{< color Olive "#808000" >}}
{{< color Navy "#000080" >}}
{{< color Purple "#800080" >}}
{{< color Teal "#008080" >}}
{{< color Silver "#c0c0c0" >}}
{{< color Gray "#808080" >}}
{{< color Red "#ff0000" >}}
{{< color Lime "#00ff00" >}}
{{< color Yellow "#ffff00" >}}
{{< color Blue "#0000ff" >}}
{{< color Fuchsia "#ff00ff" >}}
{{< color Aqua "#00ffff" >}}
{{< color White "#ffffff" >}}
{{< /color-grid >}}

## Color Cube

{{< color-grid grid-cols-6 >}}
{{< color Gray0 "#000000" >}}
{{< color NavyBlue "#00005f" >}}
{{< color DarkBlue "#000087" >}}
{{< color Blue1 "#0000af" >}}
{{< color Blue2 "#0000d7" >}}
{{< color Blue3 "#0000ff" >}}
{{< color DarkGreen "#005f00" >}}
{{< color DeepSkyBlue1 "#005f5f" >}}
{{< color DeepSkyBlue2 "#005f87" >}}
{{< color DeepSkyBlue3 "#005faf" >}}
{{< color DodgerBlue1 "#005fd7" >}}
{{< color DodgerBlue2 "#005fff" >}}
{{< color Green1 "#008700" >}}
{{< color SpringGreen1 "#00875f" >}}
{{< color Turquoise1 "#008787" >}}
{{< color DeepSkyBlue4 "#0087af" >}}
{{< color DeepSkyBlue5 "#0087d7" >}}
{{< color DodgerBlue3 "#0087ff" >}}
{{< color Green2 "#00af00" >}}
{{< color SpringGreen2 "#00af5f" >}}
{{< color DarkCyan "#00af87" >}}
{{< color LightSeaGreen "#00afaf" >}}
{{< color DeepSkyBlue6 "#00afd7" >}}
{{< color DeepSkyBlue7 "#00afff" >}}
{{< color Green3 "#00d700" >}}
{{< color SpringGreen3 "#00d75f" >}}
{{< color SpringGreen4 "#00d787" >}}
{{< color Cyan1 "#00d7af" >}}
{{< color DarkTurquoise "#00d7d7" >}}
{{< color Turquoise2 "#00d7ff" >}}
{{< color Green4 "#00ff00" >}}
{{< color SpringGreen5 "#00ff5f" >}}
{{< color SpringGreen6 "#00ff87" >}}
{{< color MediumSpringGreen "#00ffaf" >}}
{{< color Cyan2 "#00ffd7" >}}
{{< color Cyan3 "#00ffff" >}}
{{< /color-grid >}}

{{< color-grid grid-cols-6 >}}
{{< color DarkRed1 "#5f0000" >}}
{{< color DeepPink1 "#5f005f" >}}
{{< color Purple1 "#5f0087" >}}
{{< color Purple2 "#5f00af" >}}
{{< color Purple3 "#5f00d7" >}}
{{< color BlueViolet "#5f00ff" >}}
{{< color Orange1 "#5f5f00" >}}
{{< color Gray37 "#5f5f5f" >}}
{{< color MediumPurple1 "#5f5f87" >}}
{{< color SlateBlue1 "#5f5faf" >}}
{{< color SlateBlue2 "#5f5fd7" >}}
{{< color RoyalBlue "#5f5fff" >}}
{{< color Chartreuse1 "#5f8700" >}}
{{< color DarkSeaGreen1 "#5f875f" >}}
{{< color PaleTurquoise1 "#5f8787" >}}
{{< color SteelBlue1 "#5f87af" >}}
{{< color SteelBlue2 "#5f87d7" >}}
{{< color CornflowerBlue "#5f87ff" >}}
{{< color Chartreuse2 "#5faf00" >}}
{{< color DarkSeaGreen2 "#5faf5f" >}}
{{< color CadetBlue1 "#5faf87" >}}
{{< color CadetBlue2 "#5fafaf" >}}
{{< color SkyBlue1 "#5fafd7" >}}
{{< color SteelBlue3 "#5fafff" >}}
{{< color Chartreuse3 "#5fd700" >}}
{{< color PaleGreen1 "#5fd75f" >}}
{{< color SeaGreen1 "#5fd787" >}}
{{< color Aquamarine1 "#5fd7af" >}}
{{< color MediumTurquoise "#5fd7d7" >}}
{{< color SteelBlue4 "#5fd7ff" >}}
{{< color Chartreuse4 "#5fff00" >}}
{{< color SeaGreen2 "#5fff5f" >}}
{{< color SeaGreen3 "#5fff87" >}}
{{< color SeaGreen4 "#5fffaf" >}}
{{< color Aquamarine2 "#5fffd7" >}}
{{< color DarkSlateGray1 "#5fffff" >}}
{{< /color-grid >}}

{{< color-grid grid-cols-6 >}}
{{< color DarkRed2 "#870000" >}}
{{< color DeepPink2 "#87005f" >}}
{{< color DarkMagenta1 "#870087" >}}
{{< color DarkMagenta2 "#8700af" >}}
{{< color DarkViolet1 "#8700d7" >}}
{{< color Purple4 "#8700ff" >}}
{{< color Orange2 "#875f00" >}}
{{< color LightPink1 "#875f5f" >}}
{{< color Plum "#875f87" >}}
{{< color MediumPurple2 "#875faf" >}}
{{< color MediumPurple3 "#875fd7" >}}
{{< color SlateBlue3 "#875fff" >}}
{{< color Yellow1 "#878700" >}}
{{< color Wheat1 "#87875f" >}}
{{< color Gray53 "#878787" >}}
{{< color LightSlateGrey "#8787af" >}}
{{< color MediumPurple4 "#8787d7" >}}
{{< color LightSlateBlue "#8787ff" >}}
{{< color Yellow2 "#87af00" >}}
{{< color DarkOliveGreen1 "#87af5f" >}}
{{< color DarkSeaGreen3 "#87af87" >}}
{{< color LightSkyBlue1 "#87afaf" >}}
{{< color LightSkyBlue2 "#87afd7" >}}
{{< color SkyBlue2 "#87afff" >}}
{{< color Chartreuse5 "#87d700" >}}
{{< color DarkOliveGreen2 "#87d75f" >}}
{{< color PaleGreen2 "#87d787" >}}
{{< color DarkSeaGreen4 "#87d7af" >}}
{{< color DarkSlateGray2 "#87d7d7" >}}
{{< color SkyBlue3 "#87d7ff" >}}
{{< color Chartreuse6 "#87ff00" >}}
{{< color LightGreen1 "#87ff5f" >}}
{{< color LightGreen2 "#87ff87" >}}
{{< color PaleGreen3 "#87ffaf" >}}
{{< color Aquamarine3 "#87ffd7" >}}
{{< color DarkSlateGray3 "#87ffff" >}}
{{< /color-grid >}}

{{< color-grid grid-cols-6 >}}
{{< color Red1 "#af0000" >}}
{{< color DeepPink3 "#af005f" >}}
{{< color MediumVioletRed "#af0087" >}}
{{< color Magenta1 "#af00af" >}}
{{< color DarkViolet2 "#af00d7" >}}
{{< color Purple5 "#af00ff" >}}
{{< color DarkOrange1 "#af5f00" >}}
{{< color IndianRed1 "#af5f5f" >}}
{{< color HotPink1 "#af5f87" >}}
{{< color MediumOrchid1 "#af5faf" >}}
{{< color MediumOrchid2 "#af5fd7" >}}
{{< color MediumPurple5 "#af5fff" >}}
{{< color DarkGoldenrod "#af8700" >}}
{{< color LightSalmon1 "#af875f" >}}
{{< color RosyBrown "#af8787" >}}
{{< color Gray63 "#af87af" >}}
{{< color MediumPurple6 "#af87d7" >}}
{{< color MediumPurple7 "#af87ff" >}}
{{< color Gold1 "#afaf00" >}}
{{< color DarkKhaki "#afaf5f" >}}
{{< color NavajoWhite1 "#afaf87" >}}
{{< color Gray69 "#afafaf" >}}
{{< color LightSteelBlue1 "#afafd7" >}}
{{< color LightSteelBlue2 "#afafff" >}}
{{< color Yellow3 "#afd700" >}}
{{< color DarkOliveGreen3 "#afd75f" >}}
{{< color DarkSeaGreen3 "#afd787" >}}
{{< color DarkSeaGreen5 "#afd7af" >}}
{{< color LightCyan1 "#afd7d7" >}}
{{< color LightSkyBlue3 "#afd7ff" >}}
{{< color GreenYellow "#afff00" >}}
{{< color DarkOliveGreen4 "#afff5f" >}}
{{< color PaleGreen4 "#afff87" >}}
{{< color DarkSeaGreen6 "#afffaf" >}}
{{< color DarkSeaGreen7 "#afffd7" >}}
{{< color PaleTurquoise2 "#afffff" >}}
{{< /color-grid >}}

{{< color-grid grid-cols-6 >}}
{{< color Red2 "#d70000" >}}
{{< color DeepPink4 "#d7005f" >}}
{{< color DeepPink5 "#d70087" >}}
{{< color Magenta2 "#d700af" >}}
{{< color Magenta3 "#d700d7" >}}
{{< color Magenta4 "#d700ff" >}}
{{< color DarkOrange2 "#d75f00" >}}
{{< color IndianRed2 "#d75f5f" >}}
{{< color HotPink2 "#d75f87" >}}
{{< color HotPink3 "#d75faf" >}}
{{< color Orchid1 "#d75fd7" >}}
{{< color MediumOrchid3 "#d75fff" >}}
{{< color Orange3 "#d78700" >}}
{{< color LightSalmon2 "#d7875f" >}}
{{< color LightPink2 "#d78787" >}}
{{< color Pink1 "#d787af" >}}
{{< color Plum2 "#d787d7" >}}
{{< color Violet "#d787ff" >}}
{{< color Gold2 "#d7af00" >}}
{{< color LightGoldenrod1 "#d7af5f" >}}
{{< color Tan "#d7af87" >}}
{{< color MistyRose1 "#d7afaf" >}}
{{< color Thistle1 "#d7afd7" >}}
{{< color Plum3 "#d7afff" >}}
{{< color Yellow4 "#d7d700" >}}
{{< color Khaki1 "#d7d75f" >}}
{{< color LightGoldenrod2 "#d7d787" >}}
{{< color LightYellow "#d7d7af" >}}
{{< color Gray84 "#d7d7d7" >}}
{{< color LightSteelBlue3 "#d7d7ff" >}}
{{< color Yellow5 "#d7ff00" >}}
{{< color DarkOliveGreen5 "#d7ff5f" >}}
{{< color DarkOliveGreen6 "#d7ff87" >}}
{{< color DarkSeaGreen8 "#d7ffaf" >}}
{{< color Honeydew "#d7ffd7" >}}
{{< color LightCyan2 "#d7ffff" >}}
{{< /color-grid >}}

{{< color-grid grid-cols-6 >}}
{{< color Red3 "#ff0000" >}}
{{< color DeepPink6 "#ff005f" >}}
{{< color DeepPink7 "#ff0087" >}}
{{< color DeepPink8 "#ff00af" >}}
{{< color Magenta5 "#ff00d7" >}}
{{< color Magenta6 "#ff00ff" >}}
{{< color OrangeRed "#ff5f00" >}}
{{< color IndianRed3 "#ff5f5f" >}}
{{< color IndianRed4 "#ff5f87" >}}
{{< color HotPink5 "#ff5faf" >}}
{{< color HotPink6 "#ff5fd7" >}}
{{< color MediumOrchid4 "#ff5fff" >}}
{{< color DarkOrange3 "#ff8700" >}}
{{< color Salmon "#ff875f" >}}
{{< color LightCoral "#ff8787" >}}
{{< color PaleVioletRed "#ff87af" >}}
{{< color Orchid2 "#ff87d7" >}}
{{< color Orchid3 "#ff87ff" >}}
{{< color Orange4 "#ffaf00" >}}
{{< color SandyBrown "#ffaf5f" >}}
{{< color LightSalmon3 "#ffaf87" >}}
{{< color LightPink3 "#ffafaf" >}}
{{< color Pink2 "#ffafd7" >}}
{{< color Plum4 "#ffafff" >}}
{{< color Gold3 "#ffd700" >}}
{{< color LightGoldenrod3 "#ffd75f" >}}
{{< color LightGoldenrod4 "#ffd787" >}}
{{< color NavajoWhite2 "#ffd7af" >}}
{{< color MistyRose2 "#ffd7d7" >}}
{{< color Thistle2 "#ffd7ff" >}}
{{< color Yellow6 "#ffff00" >}}
{{< color LightGoldenrod5 "#ffff5f" >}}
{{< color Khaki2 "#ffff87" >}}
{{< color Wheat2 "#ffffaf" >}}
{{< color Cornsilk "#ffffd7" >}}
{{< color Gray100 "#ffffff" >}}
{{< /color-grid >}}

# Grays

{{< color-grid grid-cols-8 >}}
{{< color Gray3 "#080808" >}}
{{< color Gray7 "#121212" >}}
{{< color Gray11 "#1c1c1c" >}}
{{< color Gray15 "#262626" >}}
{{< color Gray19 "#303030" >}}
{{< color Gray23 "#3a3a3a" >}}
{{< color Gray27 "#444444" >}}
{{< color Gray30 "#4e4e4e" >}}
{{< color Gray35 "#585858" >}}
{{< color Gray39 "#626262" >}}
{{< color Gray42 "#6c6c6c" >}}
{{< color Gray46 "#767676" >}}
{{< color Gray50 "#808080" >}}
{{< color Gray54 "#8a8a8a" >}}
{{< color Gray58 "#949494" >}}
{{< color Gray62 "#9e9e9e" >}}
{{< color Gray66 "#a8a8a8" >}}
{{< color Gray70 "#b2b2b2" >}}
{{< color Gray74 "#bcbcbc" >}}
{{< color Gray78 "#c6c6c6" >}}
{{< color Gray82 "#d0d0d0" >}}
{{< color Gray85 "#dadada" >}}
{{< color Gray89 "#e4e4e4" >}}
{{< color Gray93 "#eeeeee" >}}
{{< /color-grid >}}

# Alphabetized

{{< color-grid grid-cols-6 >}}
{{< color Aqua "#00ffff" >}}
{{< color Aquamarine1 "#5fd7af" >}}
{{< color Aquamarine2 "#5fffd7" >}}
{{< color Aquamarine3 "#87ffd7" >}}
{{< color Black "#000000" >}}
{{< color Blue "#0000ff" >}}
{{< color Blue1 "#0000af" >}}
{{< color Blue2 "#0000d7" >}}
{{< color Blue3 "#0000ff" >}}
{{< color BlueViolet "#5f00ff" >}}
{{< color CadetBlue1 "#5faf87" >}}
{{< color CadetBlue2 "#5fafaf" >}}
{{< color Chartreuse1 "#5f8700" >}}
{{< color Chartreuse2 "#5faf00" >}}
{{< color Chartreuse3 "#5fd700" >}}
{{< color Chartreuse4 "#5fff00" >}}
{{< color Chartreuse5 "#87d700" >}}
{{< color Chartreuse6 "#87ff00" >}}
{{< color CornflowerBlue "#5f87ff" >}}
{{< color Cornsilk "#ffffd7" >}}
{{< color Cyan1 "#00d7af" >}}
{{< color Cyan2 "#00ffd7" >}}
{{< color Cyan3 "#00ffff" >}}
{{< color DarkBlue "#000087" >}}
{{< color DarkCyan "#00af87" >}}
{{< color DarkGoldenrod "#af8700" >}}
{{< color DarkGreen "#005f00" >}}
{{< color DarkKhaki "#afaf5f" >}}
{{< color DarkMagenta1 "#870087" >}}
{{< color DarkMagenta2 "#8700af" >}}
{{< color DarkOliveGreen1 "#87af5f" >}}
{{< color DarkOliveGreen2 "#87d75f" >}}
{{< color DarkOliveGreen3 "#afd75f" >}}
{{< color DarkOliveGreen4 "#afff5f" >}}
{{< color DarkOliveGreen5 "#d7ff5f" >}}
{{< color DarkOliveGreen6 "#d7ff87" >}}
{{< color DarkOrange1 "#af5f00" >}}
{{< color DarkOrange2 "#d75f00" >}}
{{< color DarkOrange3 "#ff8700" >}}
{{< color DarkRed1 "#5f0000" >}}
{{< color DarkRed2 "#870000" >}}
{{< color DarkSeaGreen1 "#5f875f" >}}
{{< color DarkSeaGreen2 "#5faf5f" >}}
{{< color DarkSeaGreen3 "#87af87" >}}
{{< color DarkSeaGreen3 "#afd787" >}}
{{< color DarkSeaGreen4 "#87d7af" >}}
{{< color DarkSeaGreen5 "#afd7af" >}}
{{< color DarkSeaGreen6 "#afffaf" >}}
{{< color DarkSeaGreen7 "#afffd7" >}}
{{< color DarkSeaGreen8 "#d7ffaf" >}}
{{< color DarkSlateGray1 "#5fffff" >}}
{{< color DarkSlateGray2 "#87d7d7" >}}
{{< color DarkSlateGray3 "#87ffff" >}}
{{< color DarkTurquoise "#00d7d7" >}}
{{< color DarkViolet1 "#8700d7" >}}
{{< color DarkViolet2 "#af00d7" >}}
{{< color DeepPink1 "#5f005f" >}}
{{< color DeepPink2 "#87005f" >}}
{{< color DeepPink3 "#af005f" >}}
{{< color DeepPink4 "#d7005f" >}}
{{< color DeepPink5 "#d70087" >}}
{{< color DeepPink6 "#ff005f" >}}
{{< color DeepPink7 "#ff0087" >}}
{{< color DeepPink8 "#ff00af" >}}
{{< color DeepSkyBlue1 "#005f5f" >}}
{{< color DeepSkyBlue2 "#005f87" >}}
{{< color DeepSkyBlue3 "#005faf" >}}
{{< color DeepSkyBlue4 "#0087af" >}}
{{< color DeepSkyBlue5 "#0087d7" >}}
{{< color DeepSkyBlue6 "#00afd7" >}}
{{< color DeepSkyBlue7 "#00afff" >}}
{{< color DodgerBlue1 "#005fd7" >}}
{{< color DodgerBlue2 "#005fff" >}}
{{< color DodgerBlue3 "#0087ff" >}}
{{< color Fuchsia "#ff00ff" >}}
{{< color Gold1 "#afaf00" >}}
{{< color Gold2 "#d7af00" >}}
{{< color Gold3 "#ffd700" >}}
{{< color Gray "#808080" >}}
{{< color Gray0 "#000000" >}}
{{< color Gray100 "#ffffff" >}}
{{< color Gray11 "#1c1c1c" >}}
{{< color Gray15 "#262626" >}}
{{< color Gray19 "#303030" >}}
{{< color Gray23 "#3a3a3a" >}}
{{< color Gray27 "#444444" >}}
{{< color Gray3 "#080808" >}}
{{< color Gray30 "#4e4e4e" >}}
{{< color Gray35 "#585858" >}}
{{< color Gray37 "#5f5f5f" >}}
{{< color Gray39 "#626262" >}}
{{< color Gray42 "#6c6c6c" >}}
{{< color Gray46 "#767676" >}}
{{< color Gray50 "#808080" >}}
{{< color Gray53 "#878787" >}}
{{< color Gray54 "#8a8a8a" >}}
{{< color Gray58 "#949494" >}}
{{< color Gray62 "#9e9e9e" >}}
{{< color Gray63 "#af87af" >}}
{{< color Gray66 "#a8a8a8" >}}
{{< color Gray69 "#afafaf" >}}
{{< color Gray7 "#121212" >}}
{{< color Gray70 "#b2b2b2" >}}
{{< color Gray74 "#bcbcbc" >}}
{{< color Gray78 "#c6c6c6" >}}
{{< color Gray82 "#d0d0d0" >}}
{{< color Gray84 "#d7d7d7" >}}
{{< color Gray85 "#dadada" >}}
{{< color Gray89 "#e4e4e4" >}}
{{< color Gray93 "#eeeeee" >}}
{{< color Green "#008000" >}}
{{< color Green1 "#008700" >}}
{{< color Green2 "#00af00" >}}
{{< color Green3 "#00d700" >}}
{{< color Green4 "#00ff00" >}}
{{< color GreenYellow "#afff00" >}}
{{< color Honeydew "#d7ffd7" >}}
{{< color HotPink1 "#af5f87" >}}
{{< color HotPink2 "#d75f87" >}}
{{< color HotPink3 "#d75faf" >}}
{{< color HotPink5 "#ff5faf" >}}
{{< color HotPink6 "#ff5fd7" >}}
{{< color IndianRed1 "#af5f5f" >}}
{{< color IndianRed2 "#d75f5f" >}}
{{< color IndianRed3 "#ff5f5f" >}}
{{< color IndianRed4 "#ff5f87" >}}
{{< color Khaki1 "#d7d75f" >}}
{{< color Khaki2 "#ffff87" >}}
{{< color LightCoral "#ff8787" >}}
{{< color LightCyan1 "#afd7d7" >}}
{{< color LightCyan2 "#d7ffff" >}}
{{< color LightGoldenrod1 "#d7af5f" >}}
{{< color LightGoldenrod2 "#d7d787" >}}
{{< color LightGoldenrod3 "#ffd75f" >}}
{{< color LightGoldenrod4 "#ffd787" >}}
{{< color LightGoldenrod5 "#ffff5f" >}}
{{< color LightGreen1 "#87ff5f" >}}
{{< color LightGreen2 "#87ff87" >}}
{{< color LightPink1 "#875f5f" >}}
{{< color LightPink2 "#d78787" >}}
{{< color LightPink3 "#ffafaf" >}}
{{< color LightSalmon1 "#af875f" >}}
{{< color LightSalmon2 "#d7875f" >}}
{{< color LightSalmon3 "#ffaf87" >}}
{{< color LightSeaGreen "#00afaf" >}}
{{< color LightSkyBlue1 "#87afaf" >}}
{{< color LightSkyBlue2 "#87afd7" >}}
{{< color LightSkyBlue3 "#afd7ff" >}}
{{< color LightSlateBlue "#8787ff" >}}
{{< color LightSlateGrey "#8787af" >}}
{{< color LightSteelBlue1 "#afafd7" >}}
{{< color LightSteelBlue2 "#afafff" >}}
{{< color LightSteelBlue3 "#d7d7ff" >}}
{{< color LightYellow "#d7d7af" >}}
{{< color Lime "#00ff00" >}}
{{< color Magenta1 "#af00af" >}}
{{< color Magenta2 "#d700af" >}}
{{< color Magenta3 "#d700d7" >}}
{{< color Magenta4 "#d700ff" >}}
{{< color Magenta5 "#ff00d7" >}}
{{< color Magenta6 "#ff00ff" >}}
{{< color Maroon "#800000" >}}
{{< color MediumOrchid1 "#af5faf" >}}
{{< color MediumOrchid2 "#af5fd7" >}}
{{< color MediumOrchid3 "#d75fff" >}}
{{< color MediumOrchid4 "#ff5fff" >}}
{{< color MediumPurple1 "#5f5f87" >}}
{{< color MediumPurple2 "#875faf" >}}
{{< color MediumPurple3 "#875fd7" >}}
{{< color MediumPurple4 "#8787d7" >}}
{{< color MediumPurple5 "#af5fff" >}}
{{< color MediumPurple6 "#af87d7" >}}
{{< color MediumPurple7 "#af87ff" >}}
{{< color MediumSpringGreen "#00ffaf" >}}
{{< color MediumTurquoise "#5fd7d7" >}}
{{< color MediumVioletRed "#af0087" >}}
{{< color MistyRose1 "#d7afaf" >}}
{{< color MistyRose2 "#ffd7d7" >}}
{{< color NavajoWhite1 "#afaf87" >}}
{{< color NavajoWhite2 "#ffd7af" >}}
{{< color Navy "#000080" >}}
{{< color NavyBlue "#00005f" >}}
{{< color Olive "#808000" >}}
{{< color Orange1 "#5f5f00" >}}
{{< color Orange2 "#875f00" >}}
{{< color Orange3 "#d78700" >}}
{{< color Orange4 "#ffaf00" >}}
{{< color OrangeRed "#ff5f00" >}}
{{< color Orchid1 "#d75fd7" >}}
{{< color Orchid2 "#ff87d7" >}}
{{< color Orchid3 "#ff87ff" >}}
{{< color PaleGreen1 "#5fd75f" >}}
{{< color PaleGreen2 "#87d787" >}}
{{< color PaleGreen3 "#87ffaf" >}}
{{< color PaleGreen4 "#afff87" >}}
{{< color PaleTurquoise1 "#5f8787" >}}
{{< color PaleTurquoise2 "#afffff" >}}
{{< color PaleVioletRed "#ff87af" >}}
{{< color Pink1 "#d787af" >}}
{{< color Pink2 "#ffafd7" >}}
{{< color Plum "#875f87" >}}
{{< color Plum2 "#d787d7" >}}
{{< color Plum3 "#d7afff" >}}
{{< color Plum4 "#ffafff" >}}
{{< color Purple "#800080" >}}
{{< color Purple1 "#5f0087" >}}
{{< color Purple2 "#5f00af" >}}
{{< color Purple3 "#5f00d7" >}}
{{< color Purple4 "#8700ff" >}}
{{< color Purple5 "#af00ff" >}}
{{< color Red "#ff0000" >}}
{{< color Red1 "#af0000" >}}
{{< color Red2 "#d70000" >}}
{{< color Red3 "#ff0000" >}}
{{< color RosyBrown "#af8787" >}}
{{< color RoyalBlue "#5f5fff" >}}
{{< color Salmon "#ff875f" >}}
{{< color SandyBrown "#ffaf5f" >}}
{{< color SeaGreen1 "#5fd787" >}}
{{< color SeaGreen2 "#5fff5f" >}}
{{< color SeaGreen3 "#5fff87" >}}
{{< color SeaGreen4 "#5fffaf" >}}
{{< color Silver "#c0c0c0" >}}
{{< color SkyBlue1 "#5fafd7" >}}
{{< color SkyBlue2 "#87afff" >}}
{{< color SkyBlue3 "#87d7ff" >}}
{{< color SlateBlue1 "#5f5faf" >}}
{{< color SlateBlue2 "#5f5fd7" >}}
{{< color SlateBlue3 "#875fff" >}}
{{< color SpringGreen1 "#00875f" >}}
{{< color SpringGreen2 "#00af5f" >}}
{{< color SpringGreen3 "#00d75f" >}}
{{< color SpringGreen4 "#00d787" >}}
{{< color SpringGreen5 "#00ff5f" >}}
{{< color SpringGreen6 "#00ff87" >}}
{{< color SteelBlue1 "#5f87af" >}}
{{< color SteelBlue2 "#5f87d7" >}}
{{< color SteelBlue3 "#5fafff" >}}
{{< color SteelBlue4 "#5fd7ff" >}}
{{< color Tan "#d7af87" >}}
{{< color Teal "#008080" >}}
{{< color Thistle1 "#d7afd7" >}}
{{< color Thistle2 "#ffd7ff" >}}
{{< color Turquoise1 "#008787" >}}
{{< color Turquoise2 "#00d7ff" >}}
{{< color Violet "#d787ff" >}}
{{< color Wheat1 "#87875f" >}}
{{< color Wheat2 "#ffffaf" >}}
{{< color White "#ffffff" >}}
{{< color Yellow "#ffff00" >}}
{{< color Yellow1 "#878700" >}}
{{< color Yellow2 "#87af00" >}}
{{< color Yellow3 "#afd700" >}}
{{< color Yellow4 "#d7d700" >}}
{{< color Yellow5 "#d7ff00" >}}
{{< color Yellow6 "#ffff00" >}}
{{< /color-grid >}}
