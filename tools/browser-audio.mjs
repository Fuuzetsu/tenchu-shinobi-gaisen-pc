export function installAudioProbe() {
  const probe = window.audioProbe = { contexts: [], latencyReads: 0, nonSilent: false };
  const Audio = window.AudioContext;
  Object.defineProperty(Audio.prototype, "outputLatency", {
    configurable: true,
    get() { probe.latencyReads++; return 0; },
  });
  window.AudioContext = class extends Audio {
    constructor(...args) { super(...args); probe.contexts.push(this); }
  };
  const create = Audio.prototype.createBufferSource;
  Audio.prototype.createBufferSource = function (...args) {
    const source = create.apply(this, args), start = source.start;
    source.start = function (...args) {
      if (!probe.nonSilent && this.buffer) {
        probe.nonSilent = this.buffer.getChannelData(0).some(sample => Math.abs(sample) > .001);
      }
      return start.apply(this, args);
    };
    return source;
  };
}
