import { h, Component } from 'preact';
import {Line} from 'preact-chartjs-2';
import CalendarHeatmap from 'react-calendar-heatmap';
import 'react-calendar-heatmap/dist/styles.css';
import style from 'Style/home';
import Protocol from 'Lib/protocol';
import Spinner from 'Component/spinner';
import Search from 'Component/search';
import InputSuggest from 'Component/input_suggest';

function integrate(elms) {
    let arr = [];

    let value = 0;
    for(var i in elms) {
        value += elms[i];

        arr.push(value);
    }

    return arr;
}

export default class Home extends Component {
    state = {
        heatmap: null,
        graph: null,
        events: null,
        end: null,
        start: null
    };

    componentWillMount() {
        Protocol.get_events()
            .then(x => {
                console.log(x);
                this.setState({ events: x })
            });

        Protocol.get_summarise()
            .then(x => {
                let dates = x.map(x => Date.parse(x[0]));
                let m = new Date(Math.max.apply(null,dates));
                let year = m.getFullYear();
                let month = m.getMonth();
                let plays = x.map(x => {
                    return {date: x[0], count: x[2]}
                });
                let adds = {
                    labels: x.map(x => x[0]),
                    datasets: [
                        {
                            label: 'Number of songs',
                            fill: true,
                            data: integrate(x.map(x => x[3]))
                        }
                    ]
                };

                this.setState({heatmap: plays, graph: adds, end: new Date(year, month+1, 0), start: new Date(year, month-8, 0) })
            });
    }

    render({}, { heatmap, graph, events, end, start }) {
        if(!heatmap || !events) {
            return (
                <div class={style.home}>
                    <div class={style.box}>
                        <Spinner size="40px" />
                        <br /><span class={style.loading_text}>Loading events ..</span></div>
                </div>
            );
        } else {


            return (
                <div class={style.home}>
                    <div class={style.header}>Best of</div>
                    <Search query="order:favs" />
                    <div class={style.header}>Number of plays</div>
                    <div class={style.box}>
                        <CalendarHeatmap
                            startDate={start}
                            endDate={end}
                            values={heatmap}
                            showOutOfRangeDays={true}
                        />
                    </div>
                    <div class={style.header}>Number of songs</div>
                    <div class={style.box}>
                        <Line data={graph} width="600" height="100" />
                    </div>
                    <div class={style.header}>All activities</div>
                    <div class={style.box}>
                        { events.map(x => (
                            <div>{x[0]} happened {Object.keys(x[1].action)} from {x[1].origin}</div>
                        )) }
                    </div>
                </div>
            );
        }
    }

}

