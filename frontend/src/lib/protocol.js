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

    get_playlists() {
        const uuid = guid();

        return this.send_msg(uuid, 'get_playlists', {});
    }

    async *stream(uuid, track_key) {
        while(true) {
            const buf = await this.send_msg(uuid, 'stream_next', {'key': track_key});

            if(buf.length == 0)
                break;
            else
                yield buf;
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
        //return Promise.all([].map.call(files, function(file) {
            let uuid = guid();

            let res = await self.send_msg(uuid, 'clear_buffer', null)
            .then(() => self.send_binary(file[1]))
            .then(() => self.send_msg(uuid, 'add_track', {'format': file[0]}));

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
            self.socket.addEventListener('message', function(e) {
                //console.log("Message type: " + e.type);

                if(typeof e.data === "string") {
                    var parsed = JSON.parse(e.data);

                    console.log("Got: " + e.data);

                    if(parsed.id == uuid) {
                        if(parsed.fn != fn)
                            reject("Wrong header!");
                        else {
                            if('Ok' in parsed.payload)
                                resolv(parsed.payload.Ok);
                            else if('Err' in parsed.payload)
                                reject("Got error: " + parsed.payload.Err);
                            else
                                resolv(parsed.payload);
                        }
                     }
                } else
                    resolv(new Uint8Array(e.data));

            }, {once: true});

            console.log("Send: " + proto_str);

            if(self.socket.readyState === WebSocket.OPEN)
                self.socket.send(proto_str);
            else 
                self.socket.onopen = function() {
                    self.socket.send(proto_str);
                }
        });


        return promise;
    }

    send_binary(binary) {
        var self = this;
        var promise = new Promise(function(resolv, reject) {
            //self.socket.onmessage = function(e) {
            self.socket.addEventListener('message', function(e) {

                var parsed = JSON.parse(e.data);

                console.log("Got: " + e.data);

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
