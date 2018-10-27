const path = require('path');
const autoprefixer = require('autoprefixer');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const ExtractTextPlugin = require('extract-text-webpack-plugin');
const OfflinePlugin = require('offline-plugin');
const webpack = require('webpack');
const ENV = process.env.NODE_ENV || 'development';

const CSS_MAPS = ENV!=='production';

module.exports = {
    context: path.resolve(__dirname, "src"),
    entry: './index.js',
    output: {
        path: path.resolve(__dirname, 'build'),
        filename: '[name].bundle.js',
        chunkFilename: '[name].bundle.js',
        publicPath: '/'
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
                      presets: ["es2015", "stage-2", "preact"]
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
            },
            // Emscripten JS files define a global. With `exports-loader` we can
            // load these files correctly (provided the global’s name is the same
            // as the file name).
            /*{
              test: /hex_server_protocol\.js$/,
              loader: "exports-loader"
            },
            // wasm files should not be processed but just be emitted and we want
            // to have their public URL.
            {
              test: /hex_server_protocol_bg\.wasm$/,
              type: "javascript/auto",
              loader: "file-loader",
              options: {
                publicPath: "build/"
              }
            }*/
        ]
    },
    plugins: ([
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
    ]).concat(ENV==='production' ? [
		new webpack.optimize.UglifyJsPlugin({
			output: {
				comments: false
			},
			compress: {
				unsafe_comps: true,
				properties: true,
				keep_fargs: false,
				pure_getters: true,
				collapse_vars: true,
				unsafe: true,
				warnings: false,
				screw_ie8: true,
				sequences: true,
				dead_code: true,
				drop_debugger: true,
				comparisons: true,
				conditionals: true,
				evaluate: true,
				booleans: true,
				loops: true,
				unused: true,
				hoist_funs: true,
				if_return: true,
				join_vars: true,
				cascade: true,
				drop_console: true
			}
		}),

		new OfflinePlugin({
			relativePaths: false,
			AppCache: false,
			excludes: ['_redirects'],
			ServiceWorker: {
				events: true
			},
			cacheMaps: [
				{
					match: /.*/,
					to: '/',
					requestTypes: ['navigate']
				}
			],
			publicPath: '/'
		})
    ] : []),

    devServer: {
        historyApiFallback: true,
    },
    mode: 'production'
};
