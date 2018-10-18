import { guid } from './uuid.js'
const proto = import('./hex_server_protocol');

const CALLS = {
    Search: ["query"],
    GetTrack: ["key"],
    StreamNext: ["key"],
    StreamEnd: [],
    StreamSeek: ["sample"],
    UpdateTrack: ["key", "title", "album", "interpret", "people", "composer"],
    GetSuggestion: ["key"],
    AddPlaylist: ["name"],
    DeletePlaylist: ["key"],
    SetPlaylistImage: ["key"],
    AddToPlaylist: ["key", "playlist"],
    UpdatePlaylist: ["key", "title", "desc"],
    GetPlaylists: [],
    GetPlaylist: ["key"],
    GetPlaylistsOfTrack: ["key"],
    DeleteTrack: ["key"],
    UploadYoutube: ["path"],
    UploadTrack: ["name", "format", "data"],
    VoteForTrack: ["key"],
    AskUploadProgress: [],
    GetToken: ["GetToken"],
    UpdateToken: ["token", "key", "played", "pos"],
    CreateToken: [],
    LastToken: [],
    GetSummarise: [],
    GetEvents: [],
    Download: ["format", "tracks"],
    AskDownloadProgress: []
}

class Protocol {
    constructor() {
        let self = this;
        this.pending_requests = {};
        //proto.then(x => self.proto = x);

        for(const call in CALLS) {
            // convert CamelCase to underscore_case for function calls
            const under = call.split(/(?=[A-Z])/).join('_').toLowerCase();
            this[under] = new Function(CALLS[call].join(", "), "return this.request('" + call + "', {" + CALLS[call].join(",") + "});");
        }

        this.socket = new WebSocket('ws://' + location.hostname + ':2794', 'rust-websocket');
        this.socket.binaryType = 'arraybuffer';
        this.socket.onerror = function(err) {
            console.error("Websocket error occured: " + err);
        }
        this.socket.onclose = function() {
            console.error("Connection to " + self.socket.url + " closed!");
        }
        this.socket.onmessage = function(msg) {
            const answ = new self.proto.Wrapper(new Uint8Array(msg.data));

            if(!answ)
                console.error("Could not parse answer!");

            const id = answ.id();
            if(self.pending_requests[id] == null) {
                console.log("Got packets without request!");
                return;
            }

            const [type, resolve, reject] = self.pending_requests[id];
            let action = answ.action();
                
            if(typeof action === "string") {
                reject(action);
                return;
            }

            action["packet_id"] = id;

            /*const pack_type = Object.keys(action)[0];
            console.log(pack_type);
            if(type != pack_type) {
                reject("Got packet with invalid type!");
                return;
            }*/

            resolve(action);
        }

        this.socket.onopen = function() {
            proto.then(x => {
                self.proto = x;

                /*self.request(CALL.Search, {query: "Blue"})*/
                self.search("Blue")
                    .then(x => console.log(x)).catch(err => console.error(err));
            });
        }

        return this;
    }

    dice_id() {
        return Array.from({length: 4}, () => Math.floor(Math.random() * (2 ** 32)));
    }

    request(type, param, id) {
        let req = {};
        req[type] = param;
        if(id == null)
            id = this.dice_id();

        const buf = this.proto.request_to_buf(id, req);
        
        if(!buf)
            console.error("Could not serialize packet: " + JSON.stringify(req));

        return this.await_answer(id, type, buf);
    }

    await_answer(id, type, msg) {
        let self = this;

        return new Promise((resolve, reject) => {
            this.pending_requests[id] = [type, resolve, reject];

            if(self.socket.readyState === WebSocket.OPEN)
                self.socket.send(msg.buffer);
            else 
                self.socket.addEventListener('open', function() {
                    console.log("LVDSFD");
                    self.socket.send(msg.buffer);
                }, {once: true});
        });
    }

    start_search(query) {
        const id = this.dice_id();

        return function() {
            return this.request('Search', {'query': query}, id);
        };
    }
    /*
    async *stream(track_key) {
        var first = true;
        while(true) {
            try {
                if(first) {
                    yield await this.request('stream_next', {'key': track_key});
                    first = false;
                }
                else
                    yield await this.request('stream_next', {});
            } catch(err) {
                console.log(err);
                break;
            }
        }
    }

    stream_seek(pos) {
        return this.request('stream_seek', {'pos': pos});
    }


    async upload_files(files) {
        var keys = [];
        var self = this;
        for(const file of files) {
            console.log(file);
        //return Promise.all([].map.call(files, function(file) {
            let uuid = guid();

            let res = await self.send_msg('clear_buffer', {})
            .then(() => self.send_binary(file[2]))
            .then(() => self.send_msg('upload_track', {'name': file[0], 'format': file[1]}));

            keys.push(res);
        }

        return keys;
        //}));
    }

    async get_suggestions(keys) {
        var suggestions = [];
        for(const key of keys) {
            let uuid = guid();

            let res = await this.request('get_suggestion', {'key': key});
            suggestions.push(res);
        }

        return suggestions;
    }*/
}

export default new Protocol();
