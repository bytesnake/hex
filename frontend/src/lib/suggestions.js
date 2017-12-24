export default function suggestion_flatten(obj) {
    var titles = [];
    var albums = [];
    var artists = [];

    for(var item of obj) {
        if(item.recordings && item.recordings.length > 0)
            for(var recording of item.recordings) {
                if(recording.title && titles.indexOf(recording.title) == -1)
                    titles.push(recording.title);

                for(var artist of recording.artists) {
                    if(artist.name && artists.indexOf(artist.name) == -1)
                        artists.push(artist.name);
                }

                for(var group of recording.releasegroups) {
                    if(group.title && albums.indexOf(group.title) == -1)
                        albums.push(group.title);
                }
            }
    }

    return {
        title: titles,
        album: albums,
        artist: artists
    };
}
