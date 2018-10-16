import {h, Component} from 'preact';
import style from './style.css';

export default class Spinner {
    render({size},{}) {
        return (
            <svg class={style.spinner} width={size} height={size} viewBox="0 0 66 66" xmlns="http://www.w3.org/2000/svg">
               <circle class={style.path} fill="none" stroke-width="6" stroke-linecap="round" cx="33" cy="33" r="30"></circle>
            </svg>
        );
    }
}
