import { guid } from './uuid.js'
const proto = import('./hex_server_protocol');

const CALL = {
    Search: 'Search',
    GetTrack: 'GetTrack',
    StreamNext: 'StreamNext',
    StreamEnd: 'StreamEnd',
    StreamSeek: 'StreamSeek',
    UpdateTrack: 'UpdateTrack',
    GetSuggestion: 'GetSuggestion',
    AddPlaylist: 'AddPlaylist',
    DeletePlaylist: 'DeletePlaylist',
    SetPlaylistImage: 'SetPlaylistImage',
    AddToPlaylist: 'AddToPlaylist',
    UpdatePlaylist: 'UpdatePlaylist',
    GetPlaylists: 'GetPlaylists',
    GetPlaylist: 'GetPlaylist',
    GetPlaylistsOfTrack: 'GetPlaylistsOfTrack',
    DeleteTrack: 'DeleteTrack',
    UploadTrack: 'UploadTrack',
    VoteForTrack: 'VoteForTrack',
    AskUploadProgress: 'AskUploadProgress',
    GetToken: 'GetToken',
    UpdateToken: 'UpdateToken',
    CreateToken: 'CreateToken',
    LastToken: 'LastToken',
    GetSummarise: 'GetSummarise',
    GetEvents: 'GetEvents',
    Download: 'Download',
    AskDownloadProgress: 'AskDownloadProgress'
}

class Protocol {
    constructor() {
        let self = this;
        this.pending_requests = {};
        proto.then(x => self.proto = x);

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
            const action = answ.action();
                
            if(typeof action === "string") {
                reject(action);
                return;
            }

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

                self.request(CALL.Search, {query: "Blue"})
                    .then(x => console.log(x)).catch(err => console.error(err));
            });
        }

        return this;
    }

    dice_id() {
        return Array.from({length: 4}, () => Math.floor(Math.random() * (2 ** 32)));
    }

    request(type, param) {
        let req = {};
        req[type] = param;
        const id = this.dice_id();
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

    /*
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

    get_events() {
        const uuid = guid();

        return this.send_msg(uuid, 'get_events', {});
    }

    get_summarise() {
        const uuid = guid();

        return this.send_msg(uuid, 'get_summarise', {});
    }

    update_token(token, key) {
        const uuid = guid();

        return this.send_msg(uuid, 'update_token', {'token': token, 'key': key});
    }

    last_token() {
        const uuid = guid();

        return this.send_msg(uuid, 'last_token', {});
    }

    get_token(id) {
        const uuid = guid();

        return this.send_msg(uuid, 'get_token', {'token': id});
    }

    download(uuid, format, tracks) {
        console.log("Downloading " + tracks + " in " + format);

        return this.send_msg(uuid, 'download', {'format': format, 'tracks': tracks});
    }

    ask_download_progress() {
        const uuid = guid();

        return this.send_msg(uuid, 'ask_download_progress', {});
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
    }*/
}

export default new Protocol();
