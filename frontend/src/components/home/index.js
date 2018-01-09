import { h } from 'preact';
import style from './style.less';
import Spinner from '../spinner';

export default () => {
	return (
		<div class={style.home}>
			<h1>Hex Hex Hex</h1>
			<p>That fdsa is the Home component.</p>
            <Spinner size="80px"/>
		</div>
	);
};
