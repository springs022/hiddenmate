/* @ts-self-types="./index.d.ts" */
import * as wasm from "./index_bg.wasm";
import { __wbg_set_wasm } from "./index_bg.js";

__wbg_set_wasm(wasm);

export {
    Algorithm, BackwardSearch, OneWayMateResult, Solver, check_one_way_mate, is_white_in_check, solve_known_invisible_problem, solve_variable_problem
} from "./index_bg.js";
