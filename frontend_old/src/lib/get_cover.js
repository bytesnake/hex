export default function(_artist, _album) {
    let artist = encodeURIComponent(_artist);
    let album = encodeURIComponent(_album);

    return fetch('http://ws.audioscrobbler.com/2.0/?format=json&api_key=4cb074e4b8ec4ee9ad3eb37d6f7eb240&method=album.getinfo&artist=' + artist + '&album=' + album)
    .then(x => x.json())
    .then(json => {
        if('error' in json)
            return Promise.reject('JSON error: ' + json.message);
        else {
            if(json.album && json.album.image && json.album.image.length > 0)
                return Promise.resolve(json.album.image[json.album.image.length-1]['#text']);
            else
                return Promise.reject('No cover found!');
        }
    });
}
