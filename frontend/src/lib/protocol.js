import { guid } from './uuid.js'

class Protocol {
    constructor() {
        this.socket = new WebSocket('ws://localhost:2794', 'rust-websocket');

        var self = this;
        this.promise = new Promise(function(resolve, reject) {
            self.socket.onopen = () => resolve(self);
            self.socket.onerr = () => reject();
        });

        return this;
    }

    get_song(hash) {
        var uuid = guid();

        return this.send_msg(uuid, 'get_song', {'hash': hash});
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

    send_msg(uuid, fn, payload) {
        var uuid = guid();

        var proto = {
            'id': uuid,
            'fn': fn,
            'payload': payload
        };

        var proto_str = JSON.stringify(proto);

        var self = this;
        var promise = new Promise(function(resolv, reject) {
            //self.socket.onmessage = function(e) {
            self.socket.addEventListener('message', function(e) {
                var parsed = JSON.parse(e.data);

                console.log("Got: " + e.data);

                if(parsed.id == uuid) {
                    if(parsed.fn != fn)
                        reject("Wrong header!");
                    else
                        resolv(parsed.payload);
                }
            });

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
}

export default new Protocol();
