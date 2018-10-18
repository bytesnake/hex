const path = require('path');
const autoprefixer = require('autoprefixer');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const ExtractTextPlugin = require('extract-text-webpack-plugin');
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
                // Transform our own .(less|css) files with PostCSS and CSS-modules
                test: /\.(less|css)$/,
                include: [
                    path.resolve(__dirname, 'src/components'),
                    path.resolve(__dirname, 'src/style')
                ],
                use: ExtractTextPlugin.extract({
                    fallback: 'style-loader',
                    use: [
                        {
                            loader: 'css-loader',
                            options: { modules: true, sourceMap: CSS_MAPS, importLoaders: 1, minimize: true }
                        },
                        {
                            loader: `postcss-loader`,
                            options: {
                                sourceMap: CSS_MAPS,
                                plugins: () => {
                                    autoprefixer({ browsers: [ 'last 2 versions' ] });
                                }
                            }
                        },
                        {
                            loader: 'less-loader',
                            options: { sourceMap: CSS_MAPS }
                        }
                    ]
                })
            },
            {
                test: /\.(less|css)$/,
                exclude: [
                    path.resolve(__dirname, 'src/components'),
                    path.resolve(__dirname, 'src/style')
                ],
                use: ExtractTextPlugin.extract({
                    fallback: 'style-loader',
                    use: [
                        {
                            loader: 'css-loader',
                            options: { sourceMap: CSS_MAPS, importLoaders: 1, minimize: true }
                        },
                        {
                            loader: `postcss-loader`,
                            options: {
                                sourceMap: CSS_MAPS,
                                plugins: () => {
                                    autoprefixer({ browsers: [ 'last 2 versions' ] });
                                }
                            }
                        },
                        {
                            loader: 'less-loader',
                            options: { sourceMap: CSS_MAPS }
                        }
                    ]
                })
            }
/*
            {
              test: /\.less$/,
              use: [{
                loader: 'style-loader' // creates style nodes from JS strings
              }, {
                loader: 'css-loader' // translates CSS into CommonJS
              }, {
                loader: 'less-loader' // compiles Less to CSS
              }]
            }*/
        ]
    },
    plugins: [
        new ExtractTextPlugin({
            filename: 'style.css',
            allChunks: true
            //disable: ENV !== 'production'
        }),

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
