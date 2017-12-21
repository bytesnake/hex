import { h } from 'preact';
import style from './style.less';
import { Slider } from 'preact-mdl';

export default () => {
	return (
		<div class={style.home}>
            <Slider>Test</Slider>
			<h1>Home</h1>
			<p>That fdsa is the Home component.</p>
		</div>
	);
};
