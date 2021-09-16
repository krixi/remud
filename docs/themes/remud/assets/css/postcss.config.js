const themeDir = __dirname + "../../";

module.exports = {
  plugins: [
    require("postcss-import")({
      path: [themeDir],
    }),
    require("tailwindcss")(themeDir + "css/tailwind.config.js"),
    require("autoprefixer")({
      path: [themeDir],
    }),
  ],
};
