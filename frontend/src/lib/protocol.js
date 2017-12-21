import { guid } from './uuid.js'

export default class Protocol {
    constructor() {
        this.socket = new WebSocket('ws://localhost:2794', 'rust-websocket');

        var self = this;
        return new Promise(function(resolve, reject) {
            self.socket.onopen = () => resolve(self);
            self.socket.onerr = () => reject();
        });
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

    async send_msg(uuid, fn, payload) {
        var uuid = guid();

        var proto = {
            'id': uuid,
            'fn': fn,
            'payload': payload
        };

        var self = this;
        var promise = new Promise(function(resolv, reject) {
            self.socket.onmessage = function(e) {
                var parsed = JSON.parse(e.data);

                console.log("Got: " + e.data);

                if(parsed.id == uuid) {
                    if(parsed.fn != fn)
                        reject("Wrong header!");
                    else
                        resolv(parsed.payload);
                }
            };

            var proto_str = JSON.stringify(proto);
            console.log("Send: " + proto);

            self.socket.send(proto_str);
        });


        return promise;
    }
}
