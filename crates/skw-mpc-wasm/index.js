// Note that a dynamic `import` statement here is required due to
// webpack/webpack#6615, but in theory `import { greet } from './pkg';`
// will work here one day as well!
const rust = import('./pkg');

const keygen_request = {
  "payload_id": [
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
  ],
  "payload_type": {
      "KeyGen": null
  },
  "peers": [
      [
          "12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba",
          "/ip4/10.0.0.3/tcp/2619/ws/p2p/12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba"
      ],
      [
          "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5",
          "/ip4/10.0.0.3/tcp/2620/ws/p2p/12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5"
      ],
      [
          "12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq",
          "/ip4/10.0.0.3/tcp/2621/ws/p2p/12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq"
      ]
  ],
  "sender": "12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba",
  "t": 2,
  "n": 3
}

const sign_request = {
  "payload_id": [
      1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1
  ],
  "payload_type": {
      "SignOffline": {
          "message": [
            153, 137,  54,   6, 208, 242,   9, 109,
            205, 141, 170, 237, 173, 109, 240,  83,
             63,  99, 209,  55,  95, 138, 242, 111,
            173, 209,  74,  11, 155, 198,  45, 110
          ],
          "keygen_id": [
              0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
          ],
          "keygen_peers": [
              [
                  "12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba",
                  "/ip4/10.0.0.3/tcp/2619/ws/p2p/12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba"
              ],
              [
                  "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5",
                  "/ip4/10.0.0.3/tcp/2620/ws/p2p/12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5"
              ],
              [
                  "12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq",
                  "/ip4/10.0.0.3/tcp/2621/ws/p2p/12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq"
              ]
          ]
      }
  },
  "peers": [
      [
          "12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba",
          "/ip4/10.0.0.3/tcp/2619/ws/p2p/12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba"
      ],
      [
          "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5",
          "/ip4/10.0.0.3/tcp/2620/ws/p2p/12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5"
      ]
  ],
  "sender": "12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba",
  "t": 2,
  "n": 3
}

const main = async() => {
  const {ext_run_keygen, ext_run_sign} = await rust;
  const result = await ext_run_keygen(JSON.stringify(keygen_request));
  const sign_result = await ext_run_sign(JSON.stringify(sign_request), JSON.stringify(result));
  console.log(JSON.parse(sign_result));
}

main();
