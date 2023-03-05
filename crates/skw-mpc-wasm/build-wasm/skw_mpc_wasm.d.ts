/* tslint:disable */
/* eslint-disable */
/**
* @param {string} auth_header
* @param {string} payload
* @param {string} client_identity
* @param {string} client_addr
* @param {boolean} enable_log
* @returns {Promise<string>}
*/
export function ext_run_keygen(auth_header: string, payload: string, client_identity: string, client_addr: string, enable_log: boolean): Promise<string>;
/**
* @param {string} auth_header
* @param {string} payload
* @param {string} local_key
* @param {string} client_identity
* @param {string} client_addr
* @param {boolean} enable_log
* @returns {Promise<string>}
*/
export function ext_run_sign(auth_header: string, payload: string, local_key: string, client_identity: string, client_addr: string, enable_log: boolean): Promise<string>;
