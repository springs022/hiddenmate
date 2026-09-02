const path = require("path");
const fs = require("fs");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const webpack = require("webpack");

class StaticAssetsPlugin {
  apply(compiler) {
    compiler.hooks.thisCompilation.tap("StaticAssetsPlugin", (compilation) => {
      compilation.hooks.processAssets.tap(
        {
          name: "StaticAssetsPlugin",
          stage: webpack.Compilation.PROCESS_ASSETS_STAGE_ADDITIONAL,
        },
        () => {
          const assets = [
            ["hiddenmate-ogp.png", "app/public/hiddenmate-ogp.png"],
            [
              "hiddenmate-screenshot.png",
              "app/public/hiddenmate-screenshot.png",
            ],
            ["LICENSE", "LICENSE"],
            ["THIRD_PARTY_NOTICES.md", "THIRD_PARTY_NOTICES.md"],
            ["third-party-licenses/react.txt", "node_modules/react/LICENSE"],
            [
              "third-party-licenses/react-dom.txt",
              "node_modules/react-dom/LICENSE",
            ],
            [
              "third-party-licenses/scheduler.txt",
              "node_modules/scheduler/LICENSE",
              "node_modules/.pnpm/node_modules/scheduler/LICENSE",
            ],
            [
              "third-party-licenses/bootstrap.txt",
              "node_modules/bootstrap/LICENSE",
            ],
            [
              "third-party-licenses/react-bootstrap.txt",
              "node_modules/react-bootstrap/LICENSE",
            ],
            [
              "third-party-licenses/react-icons.txt",
              "node_modules/react-icons/LICENSE",
            ],
            [
              "third-party-licenses/web-vitals.txt",
              "node_modules/web-vitals/LICENSE",
            ],
            [
              "third-party-licenses/classnames.txt",
              "node_modules/classnames/LICENSE",
              "node_modules/.pnpm/node_modules/classnames/LICENSE",
            ],
          ];

          for (const [assetName, ...sourcePaths] of assets) {
            const absolutePath = sourcePaths
              .map((sourcePath) => path.resolve(__dirname, sourcePath))
              .find((sourcePath) => fs.existsSync(sourcePath));
            if (!absolutePath) {
              throw new Error(`License file not found for ${assetName}`);
            }
            const contents = fs.readFileSync(absolutePath);
            compilation.emitAsset(
              assetName,
              new webpack.sources.RawSource(contents),
            );
          }
        },
      );
    });
  }
}

module.exports = (env, argv) => {
  const isProd = argv && argv.mode === "production";
  const configuredBasePath =
    env && typeof env.basePath === "string" ? env.basePath : undefined;
  const basePath = configuredBasePath || (isProd ? "/hiddenmate/" : "/");
  if (!basePath.startsWith("/") || !basePath.endsWith("/")) {
    throw new Error("basePath must start and end with '/'");
  }
  const previewBuild = configuredBasePath !== undefined;
  return {
    entry: "./app/src/index.tsx",
    output: {
      path: path.join(__dirname, previewBuild ? ".preview-build" : "docs"),
      filename: previewBuild ? "[contenthash].main.js" : "main.js",
      chunkFilename: previewBuild ? "[contenthash].chunk.js" : "[id].main.js",
      publicPath: basePath,
      clean: previewBuild,
    },
    module: {
      rules: [
        {
          test: /\.tsx?$/,
          use: [
            {
              loader: "babel-loader",
              options: { presets: ["@babel/preset-env", "@babel/react"] },
            },
            {
              loader: "ts-loader",
              options: {
                configFile: path.resolve(__dirname, "app/tsconfig.json"),
              },
            },
          ],
        },
        {
          test: /\.css$/,
          use: ["style-loader", "css-loader"],
        },
      ],
    },
    plugins: [
      new HtmlWebpackPlugin({
        template: "./app/public/index.html",
      }),
      new HtmlWebpackPlugin({
        template: "./app/public/index.html",
        filename: "404.html",
      }),
      new StaticAssetsPlugin(),
      new webpack.DefinePlugin({
        FMRS_API_BASE_URL: JSON.stringify(process.env.FMRS_API_BASE_URL || ""),
        FMRS_BASE_PATH: JSON.stringify(basePath),
      }),
      new WasmPackPlugin({
        crateDirectory: path.resolve(__dirname, "rust/wasm"),
        outDir: path.resolve(__dirname, "docs/pkg"),
      }),
    ],
    experiments: {
      asyncWebAssembly: true,
    },
    performance: {
      hints: false,
    },
    devServer: {
      compress: false,
      static: {
        directory: path.join(__dirname, "docs"),
      },
      port: 3000,
      historyApiFallback: true,
      proxy: [
        {
          context: ["/solve", "/fmrs_alive"],
          target: "http://127.0.0.1:1234",
          onProxyReq(proxyReq) {
            proxyReq.removeHeader("accept-encoding");
          },
        },
      ],
    },
    resolve: {
      extensions: [".ts", ".tsx", ".js", ".json"],
    },
    target: "web",
  };
};
