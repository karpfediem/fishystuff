const std = @import("std");
const zine = @import("zine");

pub fn build(b: *std.Build) !void {
    zine.website(b, .{
        .title = "Fishy Stuff - BDO Fishing Tools",
        .host_url = "https://karpfen.fish",
        .layouts_dir_path = "layouts",
        .content_dir_path = "content",
        .assets_dir_path = "assets",
        .static_assets = &.{
            "CNAME",
            // This asset is referenced in some inlined HTML in markdown
            // which Zine is not yet able to analyze so as a temporary
            // hack we mark it as a static asset.
            "favicon.ico",
            "favicon-16x16.png",
            "favicon-32x32.png",

            // Fonts referenced in CSS
            "fonts/FiraCode/FiraCode-Bold.woff",
            "fonts/FiraCode/FiraCode-Bold.woff2",
            "fonts/FiraCode/FiraCode-Light.woff",
            "fonts/FiraCode/FiraCode-Light.woff2",
            "fonts/FiraCode/FiraCode-Medium.woff",
            "fonts/FiraCode/FiraCode-Medium.woff2",
            "fonts/FiraCode/FiraCode-Regular.woff",
            "fonts/FiraCode/FiraCode-Regular.woff2",
            "fonts/FiraCode/FiraCode-SemiBold.woff",
            "fonts/FiraCode/FiraCode-SemiBold.woff2",
            "fonts/FiraCode/FiraCode-VF.woff",
            "fonts/FiraCode/FiraCode-VF.woff2",

            "fonts/Comfortaa/Comfortaa-VariableFont_wght.ttf",
            "fonts/Comfortaa/static/Comfortaa-Bold.ttf",
            "fonts/Comfortaa/static/Comfortaa-Light.ttf",
            "fonts/Comfortaa/static/Comfortaa-Medium.ttf",
            "fonts/Comfortaa/static/Comfortaa-Regular.ttf",
            "fonts/Comfortaa/static/Comfortaa-SemiBold.ttf",
            "fonts/Flavors/Flavors-Regular.ttf",
            "fonts/Modak/Modak-Regular.ttf",
            "fonts/Itim/Itim-Regular.ttf",
            "fonts/Pacifico/Pacifico-Regular.ttf",

            "fonts/jbm/JetBrainsMono-Bold.woff2",
            "fonts/jbm/JetBrainsMono-BoldItalic.woff2",
            "fonts/jbm/JetBrainsMono-ExtraBold.woff2",
            "fonts/jbm/JetBrainsMono-ExtraBoldItalic.woff2",
            "fonts/jbm/JetBrainsMono-ExtraLight.woff2",
            "fonts/jbm/JetBrainsMono-ExtraLightItalic.woff2",
            "fonts/jbm/JetBrainsMono-Italic.woff2",
            "fonts/jbm/JetBrainsMono-Light.woff2",
            "fonts/jbm/JetBrainsMono-LightItalic.woff2",
            "fonts/jbm/JetBrainsMono-Medium.woff2",
            "fonts/jbm/JetBrainsMono-MediumItalic.woff2",
            "fonts/jbm/JetBrainsMono-Regular.woff2",
            "fonts/jbm/JetBrainsMono-SemiBold.woff2",
            "fonts/jbm/JetBrainsMono-SemiBoldItalic.woff2",
            "fonts/jbm/JetBrainsMono-Thin.woff2",
            "fonts/jbm/JetBrainsMono-ThinItalic.woff2",
        },
        .build_assets = &.{
            .{
                .name = "zon",
                .lp = b.path("build.zig.zon"),
            },
            .{
                .name = "frontmatter",
                .lp = b.dependency("zine", .{}).path(
                    "frontmatter.ziggy-schema",
                ),
            },
        },
        .debug = true,
    });
}
