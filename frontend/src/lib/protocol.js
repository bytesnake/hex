import { guid } from './uuid.js'

class Protocol {
    constructor() {
        this.socket = new WebSocket('ws://127.0.0.1:2794', 'rust-websocket');
        this.socket.binaryType = 'arraybuffer';

        var self = this;
        this.promise = new Promise(function(resolve, reject) {
            self.socket.onopen = () => resolve(self);
            self.socket.onerr = () => reject();
        });

        return this;
    }

    get_track(key) {
        var uuid = guid();

        return this.send_msg(uuid, 'get_track', {'key': key});
    }

    update_track(track) {
        var uuid = guid();

        return this.send_msg(uuid, 'update_track', track);
    }

    upvote_track(key) {
        const uuid = guid();

        return this.send_msg(uuid, 'vote_for_track', {'key': key});
    }

    get_playlists() {
        const uuid = guid();

        return this.send_msg(uuid, 'get_playlists', {});
    }

    change_playlist_title(key, title) {
        const uuid = guid();
        
        return this.send_msg(uuid, 'update_playlist', {'key': key, 'title': title});
    }

    change_playlist_desc(key, desc) {
        const uuid = guid();

        return this.send_msg(uuid, 'update_playlist', {'key': key, 'desc': desc});
    }

    add_playlist(name) {
        const uuid = guid();

        return this.send_msg(uuid, 'add_playlist', {'name': name});
    }

    delete_playlist(key) {
        const uuid = guid();

        return this.send_msg(uuid, 'delete_playlist', {'key': key});
    }

    add_to_playlist(key, playlist) {
        const uuid = guid();

        return this.send_msg(uuid, 'add_to_playlist', {'key': key, 'playlist': playlist});
    }
    get_playlist(key) {
        const uuid = guid();

        return this.send_msg(uuid, 'get_playlist', {'key': key});
    }

    get_playlists_of_track(key) {
        const uuid = guid();

        return this.send_msg(uuid, 'get_playlists_of_track', {'key': key});
    }

    delete_track(key) {
        const uuid = guid();

        return this.send_msg(uuid, 'delete_track', {'key': key});
    }

    upload_youtube(path) {
        const uuid = guid();

        return this.send_msg(uuid, 'upload_youtube', {'path': path});
    }

    ask_upload_progress() {
        const uuid = guid();

        return this.send_msg(uuid, 'ask_upload_progress', {});
    }

    async *stream(uuid, track_key) {
        var first = true;
        while(true) {
            try {
                if(first) {
                    yield await this.send_msg(uuid, 'stream_next', {'key': track_key});
                    first = false;
                }
                else
                    yield await this.send_msg(uuid, 'stream_next', {});
            } catch(err) {
                console.log(err);
                break;
            }
        }
    }

    stream_seek(uuid, pos) {
        return this.send_msg(uuid, 'stream_seek', {'pos': pos});
    }

    async *search(query) {
        var uuid = guid();

        while(true) {
            const answ = await this.send_msg(uuid, 'search', {'query': query});

            for(const i of answ.answ)
                yield i;

            if(!answ.more)
                break;

        }
    }

    async upload_files(files) {
        var keys = [];
        var self = this;
        for(const file of files) {
            console.log(file);
        //return Promise.all([].map.call(files, function(file) {
            let uuid = guid();

            let res = await self.send_msg(uuid, 'clear_buffer', {})
            .then(() => self.send_binary(file[2]))
            .then(() => self.send_msg(uuid, 'upload_track', {'name': file[0], 'format': file[1]}));

            keys.push(res);
        }

        return keys;
        //}));
    }

    async get_suggestions(keys) {
        var suggestions = [];
        for(const key of keys) {
            let uuid = guid();

            let res = await this.send_msg(uuid, 'get_suggestion', {'key': key});
            suggestions.push(res);
        }

        return suggestions;
    }

    send_msg(uuid, fn, msg) {
        msg['fn'] = fn;

        var proto = {
            'id': uuid,
            'msg': msg
        };

        var proto_str = JSON.stringify(proto);

        var self = this;
        var promise = new Promise(function(resolv, reject) {
            //self.socket.onmessage = function(e) {
            self.socket.addEventListener('message', function listener(e) {

                if(typeof e.data === "string") {
                    if(e.data.startsWith("Err(")) {
                        reject("Could not parse the message!");
                        return;

                    }

                    var parsed = JSON.parse(e.data);

                    if(parsed.id == uuid) {
                        //console.log("Got: " + e.data);

                        // remove listener
                        self.socket.removeEventListener('message', listener);
                        if(parsed.fn != fn)
                            reject("Wrong header!");
                        else {
                            if(parsed.payload && 'Ok' in parsed.payload)
                                resolv(parsed.payload.Ok);
                            else if(parsed.payload && 'Err' in parsed.payload)
                                reject("Got error: " + parsed.payload.Err);
                            else
                                resolv(parsed.payload);
                        }
                     }
                } else
                    resolv(new Uint8Array(e.data));

            });

            //console.log("Send: " + proto_str);

            if(self.socket.readyState === WebSocket.OPEN)
                self.socket.send(proto_str);
            else 
                self.socket.addEventListener('open', function() {
                    self.socket.send(proto_str);
                }, {once: true});
                /*
                self.socket.onopen = function() {
                    self.socket.send(proto_str);
                }*/
        });


        return promise;
    }

    send_binary(binary) {
        var self = this;
        var promise = new Promise(function(resolv, reject) {
            //self.socket.onmessage = function(e) {
            self.socket.addEventListener('message', function(e) {

                var parsed = JSON.parse(e.data);

                //console.log("Got: " + e.data);

                if(parsed.fn != 'upload')
                    reject("Wrong header!");
                else
                    resolv();
            }, {once: true});

            if(self.socket.readyState === WebSocket.OPEN)
                self.socket.send(binary);
            else 
                self.socket.onopen = function() {
                    self.socket.send(binary);
                }
        });

        return promise;
        
    }
}

export default new Protocol();
