const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const webpack = require('webpack');
const ENV = process.env.NODE_ENV || 'development';

const CSS_MAPS = ENV!=='production';

module.exports = {
    context: path.resolve(__dirname, "src"),
    entry: './index.js',
    output: {
        path: path.resolve(__dirname, 'build'),
        filename: 'index.js'
    },
    resolve: {
        extensions: ['.jsx', '.js', '.json', '.less', 'wasm'],
        alias: {
            Lib: path.resolve(__dirname, 'src/lib/'),
            Component: path.resolve(__dirname, 'src/components/'),
            Style: path.resolve(__dirname, 'src/style/'),
            'react': 'preact-compat',
            'react-dom': 'preact-compat'

        }
    },
    module: {
        rules: [
            {
                test: /\.jsx?$/,
                exclude: path.resolve(__dirname, 'src'),
                enforce: 'pre',
                use: 'source-map-loader'
            },
            {
                test: /\.jsx?$/,
                exclude:/node_modules/,
                use: [
                  {
                    loader: 'babel-loader',
                    options: {
                      presets: ["es2015", "stage-0", "preact"]
                    }
                  }
                ]
            },
            {
                test: /\.css$/,
                use: [ 'style-loader', 'css-loader' ]
              },

            {
              test: /\.less$/,
              use: [{
                loader: 'style-loader' // creates style nodes from JS strings
              }, {
                loader: 'css-loader' // translates CSS into CommonJS
              }, {
                loader: 'less-loader' // compiles Less to CSS
              }]
            }
        ]
    },
    plugins: [
        new HtmlWebpackPlugin({
            template: './index.ejs',
            minify: { collapseWhitespace: true }
        }), 
        new CopyWebpackPlugin([
            { from: './manifest.json', to: './' },
            { from: './favicon.ico', to: './' }
        ])
    ],
    mode: 'development'
};
