// node_modules/d3-array/src/ascending.js
function ascending(a, b) {
  return a == null || b == null ? NaN : a < b ? -1 : a > b ? 1 : a >= b ? 0 : NaN;
}

// node_modules/d3-array/src/descending.js
function descending(a, b) {
  return a == null || b == null ? NaN : b < a ? -1 : b > a ? 1 : b >= a ? 0 : NaN;
}

// node_modules/d3-array/src/bisector.js
function bisector(f) {
  let compare1, compare2, delta;
  if (f.length !== 2) {
    compare1 = ascending;
    compare2 = (d, x) => ascending(f(d), x);
    delta = (d, x) => f(d) - x;
  } else {
    compare1 = f === ascending || f === descending ? f : zero;
    compare2 = f;
    delta = f;
  }
  function left(a, x, lo = 0, hi = a.length) {
    if (lo < hi) {
      if (compare1(x, x) !== 0)
        return hi;
      do {
        const mid = lo + hi >>> 1;
        if (compare2(a[mid], x) < 0)
          lo = mid + 1;
        else
          hi = mid;
      } while (lo < hi);
    }
    return lo;
  }
  function right(a, x, lo = 0, hi = a.length) {
    if (lo < hi) {
      if (compare1(x, x) !== 0)
        return hi;
      do {
        const mid = lo + hi >>> 1;
        if (compare2(a[mid], x) <= 0)
          lo = mid + 1;
        else
          hi = mid;
      } while (lo < hi);
    }
    return lo;
  }
  function center(a, x, lo = 0, hi = a.length) {
    const i = left(a, x, lo, hi - 1);
    return i > lo && delta(a[i - 1], x) > -delta(a[i], x) ? i - 1 : i;
  }
  return { left, center, right };
}
function zero() {
  return 0;
}

// node_modules/d3-array/src/number.js
function number(x) {
  return x === null ? NaN : +x;
}
function* numbers(values, valueof) {
  if (valueof === undefined) {
    for (let value of values) {
      if (value != null && (value = +value) >= value) {
        yield value;
      }
    }
  } else {
    let index = -1;
    for (let value of values) {
      if ((value = valueof(value, ++index, values)) != null && (value = +value) >= value) {
        yield value;
      }
    }
  }
}

// node_modules/d3-array/src/bisect.js
var ascendingBisect = bisector(ascending);
var bisectRight = ascendingBisect.right;
var bisectLeft = ascendingBisect.left;
var bisectCenter = bisector(number).center;
var bisect_default = bisectRight;
// node_modules/d3-array/src/blur.js
function blur(values, r) {
  if (!((r = +r) >= 0))
    throw new RangeError("invalid r");
  let length = values.length;
  if (!((length = Math.floor(length)) >= 0))
    throw new RangeError("invalid length");
  if (!length || !r)
    return values;
  const blur2 = blurf(r);
  const temp = values.slice();
  blur2(values, temp, 0, length, 1);
  blur2(temp, values, 0, length, 1);
  blur2(values, temp, 0, length, 1);
  return values;
}
var blur2 = Blur2(blurf);
var blurImage = Blur2(blurfImage);
function Blur2(blur3) {
  return function(data, rx, ry = rx) {
    if (!((rx = +rx) >= 0))
      throw new RangeError("invalid rx");
    if (!((ry = +ry) >= 0))
      throw new RangeError("invalid ry");
    let { data: values, width, height } = data;
    if (!((width = Math.floor(width)) >= 0))
      throw new RangeError("invalid width");
    if (!((height = Math.floor(height !== undefined ? height : values.length / width)) >= 0))
      throw new RangeError("invalid height");
    if (!width || !height || !rx && !ry)
      return data;
    const blurx = rx && blur3(rx);
    const blury = ry && blur3(ry);
    const temp = values.slice();
    if (blurx && blury) {
      blurh(blurx, temp, values, width, height);
      blurh(blurx, values, temp, width, height);
      blurh(blurx, temp, values, width, height);
      blurv(blury, values, temp, width, height);
      blurv(blury, temp, values, width, height);
      blurv(blury, values, temp, width, height);
    } else if (blurx) {
      blurh(blurx, values, temp, width, height);
      blurh(blurx, temp, values, width, height);
      blurh(blurx, values, temp, width, height);
    } else if (blury) {
      blurv(blury, values, temp, width, height);
      blurv(blury, temp, values, width, height);
      blurv(blury, values, temp, width, height);
    }
    return data;
  };
}
function blurh(blur3, T, S, w, h) {
  for (let y = 0, n = w * h;y < n; ) {
    blur3(T, S, y, y += w, 1);
  }
}
function blurv(blur3, T, S, w, h) {
  for (let x = 0, n = w * h;x < w; ++x) {
    blur3(T, S, x, x + n, w);
  }
}
function blurfImage(radius) {
  const blur3 = blurf(radius);
  return (T, S, start, stop, step) => {
    start <<= 2, stop <<= 2, step <<= 2;
    blur3(T, S, start + 0, stop + 0, step);
    blur3(T, S, start + 1, stop + 1, step);
    blur3(T, S, start + 2, stop + 2, step);
    blur3(T, S, start + 3, stop + 3, step);
  };
}
function blurf(radius) {
  const radius0 = Math.floor(radius);
  if (radius0 === radius)
    return bluri(radius);
  const t = radius - radius0;
  const w = 2 * radius + 1;
  return (T, S, start, stop, step) => {
    if (!((stop -= step) >= start))
      return;
    let sum = radius0 * S[start];
    const s0 = step * radius0;
    const s1 = s0 + step;
    for (let i = start, j = start + s0;i < j; i += step) {
      sum += S[Math.min(stop, i)];
    }
    for (let i = start, j = stop;i <= j; i += step) {
      sum += S[Math.min(stop, i + s0)];
      T[i] = (sum + t * (S[Math.max(start, i - s1)] + S[Math.min(stop, i + s1)])) / w;
      sum -= S[Math.max(start, i - s0)];
    }
  };
}
function bluri(radius) {
  const w = 2 * radius + 1;
  return (T, S, start, stop, step) => {
    if (!((stop -= step) >= start))
      return;
    let sum = radius * S[start];
    const s = step * radius;
    for (let i = start, j = start + s;i < j; i += step) {
      sum += S[Math.min(stop, i)];
    }
    for (let i = start, j = stop;i <= j; i += step) {
      sum += S[Math.min(stop, i + s)];
      T[i] = sum / w;
      sum -= S[Math.max(start, i - s)];
    }
  };
}
// node_modules/d3-array/src/count.js
function count(values, valueof) {
  let count2 = 0;
  if (valueof === undefined) {
    for (let value of values) {
      if (value != null && (value = +value) >= value) {
        ++count2;
      }
    }
  } else {
    let index = -1;
    for (let value of values) {
      if ((value = valueof(value, ++index, values)) != null && (value = +value) >= value) {
        ++count2;
      }
    }
  }
  return count2;
}
// node_modules/d3-array/src/cross.js
function length(array) {
  return array.length | 0;
}
function empty(length2) {
  return !(length2 > 0);
}
function arrayify(values) {
  return typeof values !== "object" || "length" in values ? values : Array.from(values);
}
function reducer(reduce) {
  return (values) => reduce(...values);
}
function cross(...values) {
  const reduce = typeof values[values.length - 1] === "function" && reducer(values.pop());
  values = values.map(arrayify);
  const lengths = values.map(length);
  const j = values.length - 1;
  const index = new Array(j + 1).fill(0);
  const product = [];
  if (j < 0 || lengths.some(empty))
    return product;
  while (true) {
    product.push(index.map((j2, i2) => values[i2][j2]));
    let i = j;
    while (++index[i] === lengths[i]) {
      if (i === 0)
        return reduce ? product.map(reduce) : product;
      index[i--] = 0;
    }
  }
}
// node_modules/d3-array/src/cumsum.js
function cumsum(values, valueof) {
  var sum = 0, index = 0;
  return Float64Array.from(values, valueof === undefined ? (v) => sum += +v || 0 : (v) => sum += +valueof(v, index++, values) || 0);
}
// node_modules/d3-array/src/variance.js
function variance(values, valueof) {
  let count2 = 0;
  let delta;
  let mean = 0;
  let sum = 0;
  if (valueof === undefined) {
    for (let value of values) {
      if (value != null && (value = +value) >= value) {
        delta = value - mean;
        mean += delta / ++count2;
        sum += delta * (value - mean);
      }
    }
  } else {
    let index = -1;
    for (let value of values) {
      if ((value = valueof(value, ++index, values)) != null && (value = +value) >= value) {
        delta = value - mean;
        mean += delta / ++count2;
        sum += delta * (value - mean);
      }
    }
  }
  if (count2 > 1)
    return sum / (count2 - 1);
}

// node_modules/d3-array/src/deviation.js
function deviation(values, valueof) {
  const v = variance(values, valueof);
  return v ? Math.sqrt(v) : v;
}
// node_modules/d3-array/src/extent.js
function extent(values, valueof) {
  let min;
  let max;
  if (valueof === undefined) {
    for (const value of values) {
      if (value != null) {
        if (min === undefined) {
          if (value >= value)
            min = max = value;
        } else {
          if (min > value)
            min = value;
          if (max < value)
            max = value;
        }
      }
    }
  } else {
    let index = -1;
    for (let value of values) {
      if ((value = valueof(value, ++index, values)) != null) {
        if (min === undefined) {
          if (value >= value)
            min = max = value;
        } else {
          if (min > value)
            min = value;
          if (max < value)
            max = value;
        }
      }
    }
  }
  return [min, max];
}
// node_modules/d3-array/src/fsum.js
class Adder {
  constructor() {
    this._partials = new Float64Array(32);
    this._n = 0;
  }
  add(x) {
    const p = this._partials;
    let i = 0;
    for (let j = 0;j < this._n && j < 32; j++) {
      const y = p[j], hi = x + y, lo = Math.abs(x) < Math.abs(y) ? x - (hi - y) : y - (hi - x);
      if (lo)
        p[i++] = lo;
      x = hi;
    }
    p[i] = x;
    this._n = i + 1;
    return this;
  }
  valueOf() {
    const p = this._partials;
    let n = this._n, x, y, lo, hi = 0;
    if (n > 0) {
      hi = p[--n];
      while (n > 0) {
        x = hi;
        y = p[--n];
        hi = x + y;
        lo = y - (hi - x);
        if (lo)
          break;
      }
      if (n > 0 && (lo < 0 && p[n - 1] < 0 || lo > 0 && p[n - 1] > 0)) {
        y = lo * 2;
        x = hi + y;
        if (y == x - hi)
          hi = x;
      }
    }
    return hi;
  }
}
function fsum(values, valueof) {
  const adder = new Adder;
  if (valueof === undefined) {
    for (let value of values) {
      if (value = +value) {
        adder.add(value);
      }
    }
  } else {
    let index = -1;
    for (let value of values) {
      if (value = +valueof(value, ++index, values)) {
        adder.add(value);
      }
    }
  }
  return +adder;
}
function fcumsum(values, valueof) {
  const adder = new Adder;
  let index = -1;
  return Float64Array.from(values, valueof === undefined ? (v) => adder.add(+v || 0) : (v) => adder.add(+valueof(v, ++index, values) || 0));
}
// node_modules/internmap/src/index.js
class InternMap extends Map {
  constructor(entries, key = keyof) {
    super();
    Object.defineProperties(this, { _intern: { value: new Map }, _key: { value: key } });
    if (entries != null)
      for (const [key2, value] of entries)
        this.set(key2, value);
  }
  get(key) {
    return super.get(intern_get(this, key));
  }
  has(key) {
    return super.has(intern_get(this, key));
  }
  set(key, value) {
    return super.set(intern_set(this, key), value);
  }
  delete(key) {
    return super.delete(intern_delete(this, key));
  }
}

class InternSet extends Set {
  constructor(values, key = keyof) {
    super();
    Object.defineProperties(this, { _intern: { value: new Map }, _key: { value: key } });
    if (values != null)
      for (const value of values)
        this.add(value);
  }
  has(value) {
    return super.has(intern_get(this, value));
  }
  add(value) {
    return super.add(intern_set(this, value));
  }
  delete(value) {
    return super.delete(intern_delete(this, value));
  }
}
function intern_get({ _intern, _key }, value) {
  const key = _key(value);
  return _intern.has(key) ? _intern.get(key) : value;
}
function intern_set({ _intern, _key }, value) {
  const key = _key(value);
  if (_intern.has(key))
    return _intern.get(key);
  _intern.set(key, value);
  return value;
}
function intern_delete({ _intern, _key }, value) {
  const key = _key(value);
  if (_intern.has(key)) {
    value = _intern.get(key);
    _intern.delete(key);
  }
  return value;
}
function keyof(value) {
  return value !== null && typeof value === "object" ? value.valueOf() : value;
}

// node_modules/d3-array/src/identity.js
function identity(x) {
  return x;
}

// node_modules/d3-array/src/group.js
function group(values, ...keys) {
  return nest(values, identity, identity, keys);
}
function groups(values, ...keys) {
  return nest(values, Array.from, identity, keys);
}
function flatten(groups2, keys) {
  for (let i = 1, n = keys.length;i < n; ++i) {
    groups2 = groups2.flatMap((g) => g.pop().map(([key, value]) => [...g, key, value]));
  }
  return groups2;
}
function flatGroup(values, ...keys) {
  return flatten(groups(values, ...keys), keys);
}
function flatRollup(values, reduce, ...keys) {
  return flatten(rollups(values, reduce, ...keys), keys);
}
function rollup(values, reduce, ...keys) {
  return nest(values, identity, reduce, keys);
}
function rollups(values, reduce, ...keys) {
  return nest(values, Array.from, reduce, keys);
}
function index(values, ...keys) {
  return nest(values, identity, unique, keys);
}
function indexes(values, ...keys) {
  return nest(values, Array.from, unique, keys);
}
function unique(values) {
  if (values.length !== 1)
    throw new Error("duplicate key");
  return values[0];
}
function nest(values, map, reduce, keys) {
  return function regroup(values2, i) {
    if (i >= keys.length)
      return reduce(values2);
    const groups2 = new InternMap;
    const keyof2 = keys[i++];
    let index2 = -1;
    for (const value of values2) {
      const key = keyof2(value, ++index2, values2);
      const group2 = groups2.get(key);
      if (group2)
        group2.push(value);
      else
        groups2.set(key, [value]);
    }
    for (const [key, values3] of groups2) {
      groups2.set(key, regroup(values3, i));
    }
    return map(groups2);
  }(values, 0);
}
// node_modules/d3-array/src/permute.js
function permute(source, keys) {
  return Array.from(keys, (key) => source[key]);
}

// node_modules/d3-array/src/sort.js
function sort(values, ...F) {
  if (typeof values[Symbol.iterator] !== "function")
    throw new TypeError("values is not iterable");
  values = Array.from(values);
  let [f] = F;
  if (f && f.length !== 2 || F.length > 1) {
    const index2 = Uint32Array.from(values, (d, i) => i);
    if (F.length > 1) {
      F = F.map((f2) => values.map(f2));
      index2.sort((i, j) => {
        for (const f2 of F) {
          const c = ascendingDefined(f2[i], f2[j]);
          if (c)
            return c;
        }
      });
    } else {
      f = values.map(f);
      index2.sort((i, j) => ascendingDefined(f[i], f[j]));
    }
    return permute(values, index2);
  }
  return values.sort(compareDefined(f));
}
function compareDefined(compare = ascending) {
  if (compare === ascending)
    return ascendingDefined;
  if (typeof compare !== "function")
    throw new TypeError("compare is not a function");
  return (a, b) => {
    const x = compare(a, b);
    if (x || x === 0)
      return x;
    return (compare(b, b) === 0) - (compare(a, a) === 0);
  };
}
function ascendingDefined(a, b) {
  return (a == null || !(a >= a)) - (b == null || !(b >= b)) || (a < b ? -1 : a > b ? 1 : 0);
}

// node_modules/d3-array/src/groupSort.js
function groupSort(values, reduce, key) {
  return (reduce.length !== 2 ? sort(rollup(values, reduce, key), ([ak, av], [bk, bv]) => ascending(av, bv) || ascending(ak, bk)) : sort(group(values, key), ([ak, av], [bk, bv]) => reduce(av, bv) || ascending(ak, bk))).map(([key2]) => key2);
}
// node_modules/d3-array/src/array.js
var array = Array.prototype;
var slice = array.slice;
var map = array.map;

// node_modules/d3-array/src/constant.js
function constant(x) {
  return () => x;
}

// node_modules/d3-array/src/ticks.js
var e10 = Math.sqrt(50);
var e5 = Math.sqrt(10);
var e2 = Math.sqrt(2);
function tickSpec(start, stop, count2) {
  const step = (stop - start) / Math.max(0, count2), power = Math.floor(Math.log10(step)), error = step / Math.pow(10, power), factor = error >= e10 ? 10 : error >= e5 ? 5 : error >= e2 ? 2 : 1;
  let i1, i2, inc;
  if (power < 0) {
    inc = Math.pow(10, -power) / factor;
    i1 = Math.round(start * inc);
    i2 = Math.round(stop * inc);
    if (i1 / inc < start)
      ++i1;
    if (i2 / inc > stop)
      --i2;
    inc = -inc;
  } else {
    inc = Math.pow(10, power) * factor;
    i1 = Math.round(start / inc);
    i2 = Math.round(stop / inc);
    if (i1 * inc < start)
      ++i1;
    if (i2 * inc > stop)
      --i2;
  }
  if (i2 < i1 && 0.5 <= count2 && count2 < 2)
    return tickSpec(start, stop, count2 * 2);
  return [i1, i2, inc];
}
function ticks(start, stop, count2) {
  stop = +stop, start = +start, count2 = +count2;
  if (!(count2 > 0))
    return [];
  if (start === stop)
    return [start];
  const reverse = stop < start, [i1, i2, inc] = reverse ? tickSpec(stop, start, count2) : tickSpec(start, stop, count2);
  if (!(i2 >= i1))
    return [];
  const n = i2 - i1 + 1, ticks2 = new Array(n);
  if (reverse) {
    if (inc < 0)
      for (let i = 0;i < n; ++i)
        ticks2[i] = (i2 - i) / -inc;
    else
      for (let i = 0;i < n; ++i)
        ticks2[i] = (i2 - i) * inc;
  } else {
    if (inc < 0)
      for (let i = 0;i < n; ++i)
        ticks2[i] = (i1 + i) / -inc;
    else
      for (let i = 0;i < n; ++i)
        ticks2[i] = (i1 + i) * inc;
  }
  return ticks2;
}
function tickIncrement(start, stop, count2) {
  stop = +stop, start = +start, count2 = +count2;
  return tickSpec(start, stop, count2)[2];
}
function tickStep(start, stop, count2) {
  stop = +stop, start = +start, count2 = +count2;
  const reverse = stop < start, inc = reverse ? tickIncrement(stop, start, count2) : tickIncrement(start, stop, count2);
  return (reverse ? -1 : 1) * (inc < 0 ? 1 / -inc : inc);
}

// node_modules/d3-array/src/nice.js
function nice(start, stop, count2) {
  let prestep;
  while (true) {
    const step = tickIncrement(start, stop, count2);
    if (step === prestep || step === 0 || !isFinite(step)) {
      return [start, stop];
    } else if (step > 0) {
      start = Math.floor(start / step) * step;
      stop = Math.ceil(stop / step) * step;
    } else if (step < 0) {
      start = Math.ceil(start * step) / step;
      stop = Math.floor(stop * step) / step;
    }
    prestep = step;
  }
}

// node_modules/d3-array/src/threshold/sturges.js
function thresholdSturges(values) {
  return Math.max(1, Math.ceil(Math.log(count(values)) / Math.LN2) + 1);
}

// node_modules/d3-array/src/bin.js
function bin() {
  var value = identity, domain = extent, threshold = thresholdSturges;
  function histogram(data) {
    if (!Array.isArray(data))
      data = Array.from(data);
    var i, n = data.length, x, step, values = new Array(n);
    for (i = 0;i < n; ++i) {
      values[i] = value(data[i], i, data);
    }
    var xz = domain(values), x0 = xz[0], x1 = xz[1], tz = threshold(values, x0, x1);
    if (!Array.isArray(tz)) {
      const max = x1, tn = +tz;
      if (domain === extent)
        [x0, x1] = nice(x0, x1, tn);
      tz = ticks(x0, x1, tn);
      if (tz[0] <= x0)
        step = tickIncrement(x0, x1, tn);
      if (tz[tz.length - 1] >= x1) {
        if (max >= x1 && domain === extent) {
          const step2 = tickIncrement(x0, x1, tn);
          if (isFinite(step2)) {
            if (step2 > 0) {
              x1 = (Math.floor(x1 / step2) + 1) * step2;
            } else if (step2 < 0) {
              x1 = (Math.ceil(x1 * -step2) + 1) / -step2;
            }
          }
        } else {
          tz.pop();
        }
      }
    }
    var m = tz.length, a = 0, b = m;
    while (tz[a] <= x0)
      ++a;
    while (tz[b - 1] > x1)
      --b;
    if (a || b < m)
      tz = tz.slice(a, b), m = b - a;
    var bins = new Array(m + 1), bin2;
    for (i = 0;i <= m; ++i) {
      bin2 = bins[i] = [];
      bin2.x0 = i > 0 ? tz[i - 1] : x0;
      bin2.x1 = i < m ? tz[i] : x1;
    }
    if (isFinite(step)) {
      if (step > 0) {
        for (i = 0;i < n; ++i) {
          if ((x = values[i]) != null && x0 <= x && x <= x1) {
            bins[Math.min(m, Math.floor((x - x0) / step))].push(data[i]);
          }
        }
      } else if (step < 0) {
        for (i = 0;i < n; ++i) {
          if ((x = values[i]) != null && x0 <= x && x <= x1) {
            const j = Math.floor((x0 - x) * step);
            bins[Math.min(m, j + (tz[j] <= x))].push(data[i]);
          }
        }
      }
    } else {
      for (i = 0;i < n; ++i) {
        if ((x = values[i]) != null && x0 <= x && x <= x1) {
          bins[bisect_default(tz, x, 0, m)].push(data[i]);
        }
      }
    }
    return bins;
  }
  histogram.value = function(_) {
    return arguments.length ? (value = typeof _ === "function" ? _ : constant(_), histogram) : value;
  };
  histogram.domain = function(_) {
    return arguments.length ? (domain = typeof _ === "function" ? _ : constant([_[0], _[1]]), histogram) : domain;
  };
  histogram.thresholds = function(_) {
    return arguments.length ? (threshold = typeof _ === "function" ? _ : constant(Array.isArray(_) ? slice.call(_) : _), histogram) : threshold;
  };
  return histogram;
}
// node_modules/d3-array/src/max.js
function max(values, valueof) {
  let max2;
  if (valueof === undefined) {
    for (const value of values) {
      if (value != null && (max2 < value || max2 === undefined && value >= value)) {
        max2 = value;
      }
    }
  } else {
    let index2 = -1;
    for (let value of values) {
      if ((value = valueof(value, ++index2, values)) != null && (max2 < value || max2 === undefined && value >= value)) {
        max2 = value;
      }
    }
  }
  return max2;
}

// node_modules/d3-array/src/maxIndex.js
function maxIndex(values, valueof) {
  let max2;
  let maxIndex2 = -1;
  let index2 = -1;
  if (valueof === undefined) {
    for (const value of values) {
      ++index2;
      if (value != null && (max2 < value || max2 === undefined && value >= value)) {
        max2 = value, maxIndex2 = index2;
      }
    }
  } else {
    for (let value of values) {
      if ((value = valueof(value, ++index2, values)) != null && (max2 < value || max2 === undefined && value >= value)) {
        max2 = value, maxIndex2 = index2;
      }
    }
  }
  return maxIndex2;
}

// node_modules/d3-array/src/min.js
function min(values, valueof) {
  let min2;
  if (valueof === undefined) {
    for (const value of values) {
      if (value != null && (min2 > value || min2 === undefined && value >= value)) {
        min2 = value;
      }
    }
  } else {
    let index2 = -1;
    for (let value of values) {
      if ((value = valueof(value, ++index2, values)) != null && (min2 > value || min2 === undefined && value >= value)) {
        min2 = value;
      }
    }
  }
  return min2;
}

// node_modules/d3-array/src/minIndex.js
function minIndex(values, valueof) {
  let min2;
  let minIndex2 = -1;
  let index2 = -1;
  if (valueof === undefined) {
    for (const value of values) {
      ++index2;
      if (value != null && (min2 > value || min2 === undefined && value >= value)) {
        min2 = value, minIndex2 = index2;
      }
    }
  } else {
    for (let value of values) {
      if ((value = valueof(value, ++index2, values)) != null && (min2 > value || min2 === undefined && value >= value)) {
        min2 = value, minIndex2 = index2;
      }
    }
  }
  return minIndex2;
}

// node_modules/d3-array/src/quickselect.js
function quickselect(array2, k, left = 0, right = Infinity, compare) {
  k = Math.floor(k);
  left = Math.floor(Math.max(0, left));
  right = Math.floor(Math.min(array2.length - 1, right));
  if (!(left <= k && k <= right))
    return array2;
  compare = compare === undefined ? ascendingDefined : compareDefined(compare);
  while (right > left) {
    if (right - left > 600) {
      const n = right - left + 1;
      const m = k - left + 1;
      const z = Math.log(n);
      const s = 0.5 * Math.exp(2 * z / 3);
      const sd = 0.5 * Math.sqrt(z * s * (n - s) / n) * (m - n / 2 < 0 ? -1 : 1);
      const newLeft = Math.max(left, Math.floor(k - m * s / n + sd));
      const newRight = Math.min(right, Math.floor(k + (n - m) * s / n + sd));
      quickselect(array2, k, newLeft, newRight, compare);
    }
    const t = array2[k];
    let i = left;
    let j = right;
    swap(array2, left, k);
    if (compare(array2[right], t) > 0)
      swap(array2, left, right);
    while (i < j) {
      swap(array2, i, j), ++i, --j;
      while (compare(array2[i], t) < 0)
        ++i;
      while (compare(array2[j], t) > 0)
        --j;
    }
    if (compare(array2[left], t) === 0)
      swap(array2, left, j);
    else
      ++j, swap(array2, j, right);
    if (j <= k)
      left = j + 1;
    if (k <= j)
      right = j - 1;
  }
  return array2;
}
function swap(array2, i, j) {
  const t = array2[i];
  array2[i] = array2[j];
  array2[j] = t;
}

// node_modules/d3-array/src/greatest.js
function greatest(values, compare = ascending) {
  let max2;
  let defined = false;
  if (compare.length === 1) {
    let maxValue;
    for (const element of values) {
      const value = compare(element);
      if (defined ? ascending(value, maxValue) > 0 : ascending(value, value) === 0) {
        max2 = element;
        maxValue = value;
        defined = true;
      }
    }
  } else {
    for (const value of values) {
      if (defined ? compare(value, max2) > 0 : compare(value, value) === 0) {
        max2 = value;
        defined = true;
      }
    }
  }
  return max2;
}

// node_modules/d3-array/src/quantile.js
function quantile(values, p, valueof) {
  values = Float64Array.from(numbers(values, valueof));
  if (!(n = values.length) || isNaN(p = +p))
    return;
  if (p <= 0 || n < 2)
    return min(values);
  if (p >= 1)
    return max(values);
  var n, i = (n - 1) * p, i0 = Math.floor(i), value0 = max(quickselect(values, i0).subarray(0, i0 + 1)), value1 = min(values.subarray(i0 + 1));
  return value0 + (value1 - value0) * (i - i0);
}
function quantileSorted(values, p, valueof = number) {
  if (!(n = values.length) || isNaN(p = +p))
    return;
  if (p <= 0 || n < 2)
    return +valueof(values[0], 0, values);
  if (p >= 1)
    return +valueof(values[n - 1], n - 1, values);
  var n, i = (n - 1) * p, i0 = Math.floor(i), value0 = +valueof(values[i0], i0, values), value1 = +valueof(values[i0 + 1], i0 + 1, values);
  return value0 + (value1 - value0) * (i - i0);
}
function quantileIndex(values, p, valueof = number) {
  if (isNaN(p = +p))
    return;
  numbers2 = Float64Array.from(values, (_, i2) => number(valueof(values[i2], i2, values)));
  if (p <= 0)
    return minIndex(numbers2);
  if (p >= 1)
    return maxIndex(numbers2);
  var numbers2, index2 = Uint32Array.from(values, (_, i2) => i2), j = numbers2.length - 1, i = Math.floor(j * p);
  quickselect(index2, i, 0, j, (i2, j2) => ascendingDefined(numbers2[i2], numbers2[j2]));
  i = greatest(index2.subarray(0, i + 1), (i2) => numbers2[i2]);
  return i >= 0 ? i : -1;
}

// node_modules/d3-array/src/threshold/freedmanDiaconis.js
function thresholdFreedmanDiaconis(values, min2, max2) {
  const c = count(values), d = quantile(values, 0.75) - quantile(values, 0.25);
  return c && d ? Math.ceil((max2 - min2) / (2 * d * Math.pow(c, -1 / 3))) : 1;
}
// node_modules/d3-array/src/threshold/scott.js
function thresholdScott(values, min2, max2) {
  const c = count(values), d = deviation(values);
  return c && d ? Math.ceil((max2 - min2) * Math.cbrt(c) / (3.49 * d)) : 1;
}
// node_modules/d3-array/src/mean.js
function mean(values, valueof) {
  let count2 = 0;
  let sum = 0;
  if (valueof === undefined) {
    for (let value of values) {
      if (value != null && (value = +value) >= value) {
        ++count2, sum += value;
      }
    }
  } else {
    let index2 = -1;
    for (let value of values) {
      if ((value = valueof(value, ++index2, values)) != null && (value = +value) >= value) {
        ++count2, sum += value;
      }
    }
  }
  if (count2)
    return sum / count2;
}
// node_modules/d3-array/src/median.js
function median(values, valueof) {
  return quantile(values, 0.5, valueof);
}
function medianIndex(values, valueof) {
  return quantileIndex(values, 0.5, valueof);
}
// node_modules/d3-array/src/merge.js
function* flatten2(arrays) {
  for (const array2 of arrays) {
    yield* array2;
  }
}
function merge(arrays) {
  return Array.from(flatten2(arrays));
}
// node_modules/d3-array/src/mode.js
function mode(values, valueof) {
  const counts = new InternMap;
  if (valueof === undefined) {
    for (let value of values) {
      if (value != null && value >= value) {
        counts.set(value, (counts.get(value) || 0) + 1);
      }
    }
  } else {
    let index2 = -1;
    for (let value of values) {
      if ((value = valueof(value, ++index2, values)) != null && value >= value) {
        counts.set(value, (counts.get(value) || 0) + 1);
      }
    }
  }
  let modeValue;
  let modeCount = 0;
  for (const [value, count2] of counts) {
    if (count2 > modeCount) {
      modeCount = count2;
      modeValue = value;
    }
  }
  return modeValue;
}
// node_modules/d3-array/src/pairs.js
function pairs(values, pairof = pair) {
  const pairs2 = [];
  let previous;
  let first = false;
  for (const value of values) {
    if (first)
      pairs2.push(pairof(previous, value));
    previous = value;
    first = true;
  }
  return pairs2;
}
function pair(a, b) {
  return [a, b];
}
// node_modules/d3-array/src/range.js
function range(start, stop, step) {
  start = +start, stop = +stop, step = (n = arguments.length) < 2 ? (stop = start, start = 0, 1) : n < 3 ? 1 : +step;
  var i = -1, n = Math.max(0, Math.ceil((stop - start) / step)) | 0, range2 = new Array(n);
  while (++i < n) {
    range2[i] = start + i * step;
  }
  return range2;
}
// node_modules/d3-array/src/rank.js
function rank(values, valueof = ascending) {
  if (typeof values[Symbol.iterator] !== "function")
    throw new TypeError("values is not iterable");
  let V = Array.from(values);
  const R = new Float64Array(V.length);
  if (valueof.length !== 2)
    V = V.map(valueof), valueof = ascending;
  const compareIndex = (i, j) => valueof(V[i], V[j]);
  let k, r;
  values = Uint32Array.from(V, (_, i) => i);
  values.sort(valueof === ascending ? (i, j) => ascendingDefined(V[i], V[j]) : compareDefined(compareIndex));
  values.forEach((j, i) => {
    const c = compareIndex(j, k === undefined ? j : k);
    if (c >= 0) {
      if (k === undefined || c > 0)
        k = j, r = i;
      R[j] = r;
    } else {
      R[j] = NaN;
    }
  });
  return R;
}
// node_modules/d3-array/src/least.js
function least(values, compare = ascending) {
  let min2;
  let defined = false;
  if (compare.length === 1) {
    let minValue;
    for (const element of values) {
      const value = compare(element);
      if (defined ? ascending(value, minValue) < 0 : ascending(value, value) === 0) {
        min2 = element;
        minValue = value;
        defined = true;
      }
    }
  } else {
    for (const value of values) {
      if (defined ? compare(value, min2) < 0 : compare(value, value) === 0) {
        min2 = value;
        defined = true;
      }
    }
  }
  return min2;
}
// node_modules/d3-array/src/leastIndex.js
function leastIndex(values, compare = ascending) {
  if (compare.length === 1)
    return minIndex(values, compare);
  let minValue;
  let min2 = -1;
  let index2 = -1;
  for (const value of values) {
    ++index2;
    if (min2 < 0 ? compare(value, value) === 0 : compare(value, minValue) < 0) {
      minValue = value;
      min2 = index2;
    }
  }
  return min2;
}
// node_modules/d3-array/src/greatestIndex.js
function greatestIndex(values, compare = ascending) {
  if (compare.length === 1)
    return maxIndex(values, compare);
  let maxValue;
  let max2 = -1;
  let index2 = -1;
  for (const value of values) {
    ++index2;
    if (max2 < 0 ? compare(value, value) === 0 : compare(value, maxValue) > 0) {
      maxValue = value;
      max2 = index2;
    }
  }
  return max2;
}
// node_modules/d3-array/src/scan.js
function scan(values, compare) {
  const index2 = leastIndex(values, compare);
  return index2 < 0 ? undefined : index2;
}
// node_modules/d3-array/src/shuffle.js
var shuffle_default = shuffler(Math.random);
function shuffler(random) {
  return function shuffle(array2, i0 = 0, i1 = array2.length) {
    let m = i1 - (i0 = +i0);
    while (m) {
      const i = random() * m-- | 0, t = array2[m + i0];
      array2[m + i0] = array2[i + i0];
      array2[i + i0] = t;
    }
    return array2;
  };
}
// node_modules/d3-array/src/sum.js
function sum(values, valueof) {
  let sum2 = 0;
  if (valueof === undefined) {
    for (let value of values) {
      if (value = +value) {
        sum2 += value;
      }
    }
  } else {
    let index2 = -1;
    for (let value of values) {
      if (value = +valueof(value, ++index2, values)) {
        sum2 += value;
      }
    }
  }
  return sum2;
}
// node_modules/d3-array/src/transpose.js
function transpose(matrix) {
  if (!(n = matrix.length))
    return [];
  for (var i = -1, m = min(matrix, length2), transpose2 = new Array(m);++i < m; ) {
    for (var j = -1, n, row = transpose2[i] = new Array(n);++j < n; ) {
      row[j] = matrix[j][i];
    }
  }
  return transpose2;
}
function length2(d) {
  return d.length;
}
// node_modules/d3-array/src/zip.js
function zip() {
  return transpose(arguments);
}
// node_modules/d3-array/src/every.js
function every(values, test) {
  if (typeof test !== "function")
    throw new TypeError("test is not a function");
  let index2 = -1;
  for (const value of values) {
    if (!test(value, ++index2, values)) {
      return false;
    }
  }
  return true;
}
// node_modules/d3-array/src/some.js
function some(values, test) {
  if (typeof test !== "function")
    throw new TypeError("test is not a function");
  let index2 = -1;
  for (const value of values) {
    if (test(value, ++index2, values)) {
      return true;
    }
  }
  return false;
}
// node_modules/d3-array/src/filter.js
function filter(values, test) {
  if (typeof test !== "function")
    throw new TypeError("test is not a function");
  const array2 = [];
  let index2 = -1;
  for (const value of values) {
    if (test(value, ++index2, values)) {
      array2.push(value);
    }
  }
  return array2;
}
// node_modules/d3-array/src/map.js
function map2(values, mapper) {
  if (typeof values[Symbol.iterator] !== "function")
    throw new TypeError("values is not iterable");
  if (typeof mapper !== "function")
    throw new TypeError("mapper is not a function");
  return Array.from(values, (value, index2) => mapper(value, index2, values));
}
// node_modules/d3-array/src/reduce.js
function reduce(values, reducer2, value) {
  if (typeof reducer2 !== "function")
    throw new TypeError("reducer is not a function");
  const iterator = values[Symbol.iterator]();
  let done, next, index2 = -1;
  if (arguments.length < 3) {
    ({ done, value } = iterator.next());
    if (done)
      return;
    ++index2;
  }
  while ({ done, value: next } = iterator.next(), !done) {
    value = reducer2(value, next, ++index2, values);
  }
  return value;
}
// node_modules/d3-array/src/reverse.js
function reverse(values) {
  if (typeof values[Symbol.iterator] !== "function")
    throw new TypeError("values is not iterable");
  return Array.from(values).reverse();
}
// node_modules/d3-array/src/difference.js
function difference(values, ...others) {
  values = new InternSet(values);
  for (const other of others) {
    for (const value of other) {
      values.delete(value);
    }
  }
  return values;
}
// node_modules/d3-array/src/disjoint.js
function disjoint(values, other) {
  const iterator = other[Symbol.iterator](), set = new InternSet;
  for (const v of values) {
    if (set.has(v))
      return false;
    let value, done;
    while ({ value, done } = iterator.next()) {
      if (done)
        break;
      if (Object.is(v, value))
        return false;
      set.add(value);
    }
  }
  return true;
}
// node_modules/d3-array/src/intersection.js
function intersection(values, ...others) {
  values = new InternSet(values);
  others = others.map(set);
  out:
    for (const value of values) {
      for (const other of others) {
        if (!other.has(value)) {
          values.delete(value);
          continue out;
        }
      }
    }
  return values;
}
function set(values) {
  return values instanceof InternSet ? values : new InternSet(values);
}
// node_modules/d3-array/src/superset.js
function superset(values, other) {
  const iterator = values[Symbol.iterator](), set2 = new Set;
  for (const o of other) {
    const io = intern(o);
    if (set2.has(io))
      continue;
    let value, done;
    while ({ value, done } = iterator.next()) {
      if (done)
        return false;
      const ivalue = intern(value);
      set2.add(ivalue);
      if (Object.is(io, ivalue))
        break;
    }
  }
  return true;
}
function intern(value) {
  return value !== null && typeof value === "object" ? value.valueOf() : value;
}

// node_modules/d3-array/src/subset.js
function subset(values, other) {
  return superset(other, values);
}
// node_modules/d3-array/src/union.js
function union(...others) {
  const set2 = new InternSet;
  for (const other of others) {
    for (const o of other) {
      set2.add(o);
    }
  }
  return set2;
}
// node_modules/d3-axis/src/identity.js
function identity_default(x) {
  return x;
}

// node_modules/d3-axis/src/axis.js
var top = 1;
var right = 2;
var bottom = 3;
var left = 4;
var epsilon = 0.000001;
function translateX(x) {
  return "translate(" + x + ",0)";
}
function translateY(y) {
  return "translate(0," + y + ")";
}
function number2(scale) {
  return (d) => +scale(d);
}
function center(scale, offset) {
  offset = Math.max(0, scale.bandwidth() - offset * 2) / 2;
  if (scale.round())
    offset = Math.round(offset);
  return (d) => +scale(d) + offset;
}
function entering() {
  return !this.__axis;
}
function axis(orient, scale) {
  var tickArguments = [], tickValues = null, tickFormat = null, tickSizeInner = 6, tickSizeOuter = 6, tickPadding = 3, offset = typeof window !== "undefined" && window.devicePixelRatio > 1 ? 0 : 0.5, k = orient === top || orient === left ? -1 : 1, x = orient === left || orient === right ? "x" : "y", transform = orient === top || orient === bottom ? translateX : translateY;
  function axis2(context) {
    var values = tickValues == null ? scale.ticks ? scale.ticks.apply(scale, tickArguments) : scale.domain() : tickValues, format = tickFormat == null ? scale.tickFormat ? scale.tickFormat.apply(scale, tickArguments) : identity_default : tickFormat, spacing = Math.max(tickSizeInner, 0) + tickPadding, range2 = scale.range(), range0 = +range2[0] + offset, range1 = +range2[range2.length - 1] + offset, position = (scale.bandwidth ? center : number2)(scale.copy(), offset), selection = context.selection ? context.selection() : context, path = selection.selectAll(".domain").data([null]), tick = selection.selectAll(".tick").data(values, scale).order(), tickExit = tick.exit(), tickEnter = tick.enter().append("g").attr("class", "tick"), line = tick.select("line"), text = tick.select("text");
    path = path.merge(path.enter().insert("path", ".tick").attr("class", "domain").attr("stroke", "currentColor"));
    tick = tick.merge(tickEnter);
    line = line.merge(tickEnter.append("line").attr("stroke", "currentColor").attr(x + "2", k * tickSizeInner));
    text = text.merge(tickEnter.append("text").attr("fill", "currentColor").attr(x, k * spacing).attr("dy", orient === top ? "0em" : orient === bottom ? "0.71em" : "0.32em"));
    if (context !== selection) {
      path = path.transition(context);
      tick = tick.transition(context);
      line = line.transition(context);
      text = text.transition(context);
      tickExit = tickExit.transition(context).attr("opacity", epsilon).attr("transform", function(d) {
        return isFinite(d = position(d)) ? transform(d + offset) : this.getAttribute("transform");
      });
      tickEnter.attr("opacity", epsilon).attr("transform", function(d) {
        var p = this.parentNode.__axis;
        return transform((p && isFinite(p = p(d)) ? p : position(d)) + offset);
      });
    }
    tickExit.remove();
    path.attr("d", orient === left || orient === right ? tickSizeOuter ? "M" + k * tickSizeOuter + "," + range0 + "H" + offset + "V" + range1 + "H" + k * tickSizeOuter : "M" + offset + "," + range0 + "V" + range1 : tickSizeOuter ? "M" + range0 + "," + k * tickSizeOuter + "V" + offset + "H" + range1 + "V" + k * tickSizeOuter : "M" + range0 + "," + offset + "H" + range1);
    tick.attr("opacity", 1).attr("transform", function(d) {
      return transform(position(d) + offset);
    });
    line.attr(x + "2", k * tickSizeInner);
    text.attr(x, k * spacing).text(format);
    selection.filter(entering).attr("fill", "none").attr("font-size", 10).attr("font-family", "sans-serif").attr("text-anchor", orient === right ? "start" : orient === left ? "end" : "middle");
    selection.each(function() {
      this.__axis = position;
    });
  }
  axis2.scale = function(_) {
    return arguments.length ? (scale = _, axis2) : scale;
  };
  axis2.ticks = function() {
    return tickArguments = Array.from(arguments), axis2;
  };
  axis2.tickArguments = function(_) {
    return arguments.length ? (tickArguments = _ == null ? [] : Array.from(_), axis2) : tickArguments.slice();
  };
  axis2.tickValues = function(_) {
    return arguments.length ? (tickValues = _ == null ? null : Array.from(_), axis2) : tickValues && tickValues.slice();
  };
  axis2.tickFormat = function(_) {
    return arguments.length ? (tickFormat = _, axis2) : tickFormat;
  };
  axis2.tickSize = function(_) {
    return arguments.length ? (tickSizeInner = tickSizeOuter = +_, axis2) : tickSizeInner;
  };
  axis2.tickSizeInner = function(_) {
    return arguments.length ? (tickSizeInner = +_, axis2) : tickSizeInner;
  };
  axis2.tickSizeOuter = function(_) {
    return arguments.length ? (tickSizeOuter = +_, axis2) : tickSizeOuter;
  };
  axis2.tickPadding = function(_) {
    return arguments.length ? (tickPadding = +_, axis2) : tickPadding;
  };
  axis2.offset = function(_) {
    return arguments.length ? (offset = +_, axis2) : offset;
  };
  return axis2;
}
function axisTop(scale) {
  return axis(top, scale);
}
function axisRight(scale) {
  return axis(right, scale);
}
function axisBottom(scale) {
  return axis(bottom, scale);
}
function axisLeft(scale) {
  return axis(left, scale);
}
// node_modules/d3-dispatch/src/dispatch.js
var noop = { value: () => {} };
function dispatch() {
  for (var i = 0, n = arguments.length, _ = {}, t;i < n; ++i) {
    if (!(t = arguments[i] + "") || t in _ || /[\s.]/.test(t))
      throw new Error("illegal type: " + t);
    _[t] = [];
  }
  return new Dispatch(_);
}
function Dispatch(_) {
  this._ = _;
}
function parseTypenames(typenames, types) {
  return typenames.trim().split(/^|\s+/).map(function(t) {
    var name = "", i = t.indexOf(".");
    if (i >= 0)
      name = t.slice(i + 1), t = t.slice(0, i);
    if (t && !types.hasOwnProperty(t))
      throw new Error("unknown type: " + t);
    return { type: t, name };
  });
}
Dispatch.prototype = dispatch.prototype = {
  constructor: Dispatch,
  on: function(typename, callback) {
    var _ = this._, T = parseTypenames(typename + "", _), t, i = -1, n = T.length;
    if (arguments.length < 2) {
      while (++i < n)
        if ((t = (typename = T[i]).type) && (t = get(_[t], typename.name)))
          return t;
      return;
    }
    if (callback != null && typeof callback !== "function")
      throw new Error("invalid callback: " + callback);
    while (++i < n) {
      if (t = (typename = T[i]).type)
        _[t] = set2(_[t], typename.name, callback);
      else if (callback == null)
        for (t in _)
          _[t] = set2(_[t], typename.name, null);
    }
    return this;
  },
  copy: function() {
    var copy = {}, _ = this._;
    for (var t in _)
      copy[t] = _[t].slice();
    return new Dispatch(copy);
  },
  call: function(type, that) {
    if ((n = arguments.length - 2) > 0)
      for (var args = new Array(n), i = 0, n, t;i < n; ++i)
        args[i] = arguments[i + 2];
    if (!this._.hasOwnProperty(type))
      throw new Error("unknown type: " + type);
    for (t = this._[type], i = 0, n = t.length;i < n; ++i)
      t[i].value.apply(that, args);
  },
  apply: function(type, that, args) {
    if (!this._.hasOwnProperty(type))
      throw new Error("unknown type: " + type);
    for (var t = this._[type], i = 0, n = t.length;i < n; ++i)
      t[i].value.apply(that, args);
  }
};
function get(type, name) {
  for (var i = 0, n = type.length, c;i < n; ++i) {
    if ((c = type[i]).name === name) {
      return c.value;
    }
  }
}
function set2(type, name, callback) {
  for (var i = 0, n = type.length;i < n; ++i) {
    if (type[i].name === name) {
      type[i] = noop, type = type.slice(0, i).concat(type.slice(i + 1));
      break;
    }
  }
  if (callback != null)
    type.push({ name, value: callback });
  return type;
}
var dispatch_default = dispatch;
// node_modules/d3-selection/src/namespaces.js
var xhtml = "http://www.w3.org/1999/xhtml";
var namespaces_default = {
  svg: "http://www.w3.org/2000/svg",
  xhtml,
  xlink: "http://www.w3.org/1999/xlink",
  xml: "http://www.w3.org/XML/1998/namespace",
  xmlns: "http://www.w3.org/2000/xmlns/"
};

// node_modules/d3-selection/src/namespace.js
function namespace_default(name) {
  var prefix = name += "", i = prefix.indexOf(":");
  if (i >= 0 && (prefix = name.slice(0, i)) !== "xmlns")
    name = name.slice(i + 1);
  return namespaces_default.hasOwnProperty(prefix) ? { space: namespaces_default[prefix], local: name } : name;
}

// node_modules/d3-selection/src/creator.js
function creatorInherit(name) {
  return function() {
    var document2 = this.ownerDocument, uri = this.namespaceURI;
    return uri === xhtml && document2.documentElement.namespaceURI === xhtml ? document2.createElement(name) : document2.createElementNS(uri, name);
  };
}
function creatorFixed(fullname) {
  return function() {
    return this.ownerDocument.createElementNS(fullname.space, fullname.local);
  };
}
function creator_default(name) {
  var fullname = namespace_default(name);
  return (fullname.local ? creatorFixed : creatorInherit)(fullname);
}

// node_modules/d3-selection/src/selector.js
function none() {}
function selector_default(selector) {
  return selector == null ? none : function() {
    return this.querySelector(selector);
  };
}

// node_modules/d3-selection/src/selection/select.js
function select_default(select) {
  if (typeof select !== "function")
    select = selector_default(select);
  for (var groups2 = this._groups, m = groups2.length, subgroups = new Array(m), j = 0;j < m; ++j) {
    for (var group2 = groups2[j], n = group2.length, subgroup = subgroups[j] = new Array(n), node, subnode, i = 0;i < n; ++i) {
      if ((node = group2[i]) && (subnode = select.call(node, node.__data__, i, group2))) {
        if ("__data__" in node)
          subnode.__data__ = node.__data__;
        subgroup[i] = subnode;
      }
    }
  }
  return new Selection(subgroups, this._parents);
}

// node_modules/d3-selection/src/array.js
function array2(x) {
  return x == null ? [] : Array.isArray(x) ? x : Array.from(x);
}

// node_modules/d3-selection/src/selectorAll.js
function empty2() {
  return [];
}
function selectorAll_default(selector) {
  return selector == null ? empty2 : function() {
    return this.querySelectorAll(selector);
  };
}

// node_modules/d3-selection/src/selection/selectAll.js
function arrayAll(select) {
  return function() {
    return array2(select.apply(this, arguments));
  };
}
function selectAll_default(select) {
  if (typeof select === "function")
    select = arrayAll(select);
  else
    select = selectorAll_default(select);
  for (var groups2 = this._groups, m = groups2.length, subgroups = [], parents = [], j = 0;j < m; ++j) {
    for (var group2 = groups2[j], n = group2.length, node, i = 0;i < n; ++i) {
      if (node = group2[i]) {
        subgroups.push(select.call(node, node.__data__, i, group2));
        parents.push(node);
      }
    }
  }
  return new Selection(subgroups, parents);
}

// node_modules/d3-selection/src/matcher.js
function matcher_default(selector) {
  return function() {
    return this.matches(selector);
  };
}
function childMatcher(selector) {
  return function(node) {
    return node.matches(selector);
  };
}

// node_modules/d3-selection/src/selection/selectChild.js
var find = Array.prototype.find;
function childFind(match) {
  return function() {
    return find.call(this.children, match);
  };
}
function childFirst() {
  return this.firstElementChild;
}
function selectChild_default(match) {
  return this.select(match == null ? childFirst : childFind(typeof match === "function" ? match : childMatcher(match)));
}

// node_modules/d3-selection/src/selection/selectChildren.js
var filter2 = Array.prototype.filter;
function children() {
  return Array.from(this.children);
}
function childrenFilter(match) {
  return function() {
    return filter2.call(this.children, match);
  };
}
function selectChildren_default(match) {
  return this.selectAll(match == null ? children : childrenFilter(typeof match === "function" ? match : childMatcher(match)));
}

// node_modules/d3-selection/src/selection/filter.js
function filter_default(match) {
  if (typeof match !== "function")
    match = matcher_default(match);
  for (var groups2 = this._groups, m = groups2.length, subgroups = new Array(m), j = 0;j < m; ++j) {
    for (var group2 = groups2[j], n = group2.length, subgroup = subgroups[j] = [], node, i = 0;i < n; ++i) {
      if ((node = group2[i]) && match.call(node, node.__data__, i, group2)) {
        subgroup.push(node);
      }
    }
  }
  return new Selection(subgroups, this._parents);
}

// node_modules/d3-selection/src/selection/sparse.js
function sparse_default(update) {
  return new Array(update.length);
}

// node_modules/d3-selection/src/selection/enter.js
function enter_default() {
  return new Selection(this._enter || this._groups.map(sparse_default), this._parents);
}
function EnterNode(parent, datum) {
  this.ownerDocument = parent.ownerDocument;
  this.namespaceURI = parent.namespaceURI;
  this._next = null;
  this._parent = parent;
  this.__data__ = datum;
}
EnterNode.prototype = {
  constructor: EnterNode,
  appendChild: function(child) {
    return this._parent.insertBefore(child, this._next);
  },
  insertBefore: function(child, next) {
    return this._parent.insertBefore(child, next);
  },
  querySelector: function(selector) {
    return this._parent.querySelector(selector);
  },
  querySelectorAll: function(selector) {
    return this._parent.querySelectorAll(selector);
  }
};

// node_modules/d3-selection/src/constant.js
function constant_default(x) {
  return function() {
    return x;
  };
}

// node_modules/d3-selection/src/selection/data.js
function bindIndex(parent, group2, enter, update, exit, data) {
  var i = 0, node, groupLength = group2.length, dataLength = data.length;
  for (;i < dataLength; ++i) {
    if (node = group2[i]) {
      node.__data__ = data[i];
      update[i] = node;
    } else {
      enter[i] = new EnterNode(parent, data[i]);
    }
  }
  for (;i < groupLength; ++i) {
    if (node = group2[i]) {
      exit[i] = node;
    }
  }
}
function bindKey(parent, group2, enter, update, exit, data, key) {
  var i, node, nodeByKeyValue = new Map, groupLength = group2.length, dataLength = data.length, keyValues = new Array(groupLength), keyValue;
  for (i = 0;i < groupLength; ++i) {
    if (node = group2[i]) {
      keyValues[i] = keyValue = key.call(node, node.__data__, i, group2) + "";
      if (nodeByKeyValue.has(keyValue)) {
        exit[i] = node;
      } else {
        nodeByKeyValue.set(keyValue, node);
      }
    }
  }
  for (i = 0;i < dataLength; ++i) {
    keyValue = key.call(parent, data[i], i, data) + "";
    if (node = nodeByKeyValue.get(keyValue)) {
      update[i] = node;
      node.__data__ = data[i];
      nodeByKeyValue.delete(keyValue);
    } else {
      enter[i] = new EnterNode(parent, data[i]);
    }
  }
  for (i = 0;i < groupLength; ++i) {
    if ((node = group2[i]) && nodeByKeyValue.get(keyValues[i]) === node) {
      exit[i] = node;
    }
  }
}
function datum(node) {
  return node.__data__;
}
function data_default(value, key) {
  if (!arguments.length)
    return Array.from(this, datum);
  var bind = key ? bindKey : bindIndex, parents = this._parents, groups2 = this._groups;
  if (typeof value !== "function")
    value = constant_default(value);
  for (var m = groups2.length, update = new Array(m), enter = new Array(m), exit = new Array(m), j = 0;j < m; ++j) {
    var parent = parents[j], group2 = groups2[j], groupLength = group2.length, data = arraylike(value.call(parent, parent && parent.__data__, j, parents)), dataLength = data.length, enterGroup = enter[j] = new Array(dataLength), updateGroup = update[j] = new Array(dataLength), exitGroup = exit[j] = new Array(groupLength);
    bind(parent, group2, enterGroup, updateGroup, exitGroup, data, key);
    for (var i0 = 0, i1 = 0, previous, next;i0 < dataLength; ++i0) {
      if (previous = enterGroup[i0]) {
        if (i0 >= i1)
          i1 = i0 + 1;
        while (!(next = updateGroup[i1]) && ++i1 < dataLength)
          ;
        previous._next = next || null;
      }
    }
  }
  update = new Selection(update, parents);
  update._enter = enter;
  update._exit = exit;
  return update;
}
function arraylike(data) {
  return typeof data === "object" && "length" in data ? data : Array.from(data);
}

// node_modules/d3-selection/src/selection/exit.js
function exit_default() {
  return new Selection(this._exit || this._groups.map(sparse_default), this._parents);
}

// node_modules/d3-selection/src/selection/join.js
function join_default(onenter, onupdate, onexit) {
  var enter = this.enter(), update = this, exit = this.exit();
  if (typeof onenter === "function") {
    enter = onenter(enter);
    if (enter)
      enter = enter.selection();
  } else {
    enter = enter.append(onenter + "");
  }
  if (onupdate != null) {
    update = onupdate(update);
    if (update)
      update = update.selection();
  }
  if (onexit == null)
    exit.remove();
  else
    onexit(exit);
  return enter && update ? enter.merge(update).order() : update;
}

// node_modules/d3-selection/src/selection/merge.js
function merge_default(context) {
  var selection = context.selection ? context.selection() : context;
  for (var groups0 = this._groups, groups1 = selection._groups, m0 = groups0.length, m1 = groups1.length, m = Math.min(m0, m1), merges = new Array(m0), j = 0;j < m; ++j) {
    for (var group0 = groups0[j], group1 = groups1[j], n = group0.length, merge2 = merges[j] = new Array(n), node, i = 0;i < n; ++i) {
      if (node = group0[i] || group1[i]) {
        merge2[i] = node;
      }
    }
  }
  for (;j < m0; ++j) {
    merges[j] = groups0[j];
  }
  return new Selection(merges, this._parents);
}

// node_modules/d3-selection/src/selection/order.js
function order_default() {
  for (var groups2 = this._groups, j = -1, m = groups2.length;++j < m; ) {
    for (var group2 = groups2[j], i = group2.length - 1, next = group2[i], node;--i >= 0; ) {
      if (node = group2[i]) {
        if (next && node.compareDocumentPosition(next) ^ 4)
          next.parentNode.insertBefore(node, next);
        next = node;
      }
    }
  }
  return this;
}

// node_modules/d3-selection/src/selection/sort.js
function sort_default(compare) {
  if (!compare)
    compare = ascending2;
  function compareNode(a, b) {
    return a && b ? compare(a.__data__, b.__data__) : !a - !b;
  }
  for (var groups2 = this._groups, m = groups2.length, sortgroups = new Array(m), j = 0;j < m; ++j) {
    for (var group2 = groups2[j], n = group2.length, sortgroup = sortgroups[j] = new Array(n), node, i = 0;i < n; ++i) {
      if (node = group2[i]) {
        sortgroup[i] = node;
      }
    }
    sortgroup.sort(compareNode);
  }
  return new Selection(sortgroups, this._parents).order();
}
function ascending2(a, b) {
  return a < b ? -1 : a > b ? 1 : a >= b ? 0 : NaN;
}

// node_modules/d3-selection/src/selection/call.js
function call_default() {
  var callback = arguments[0];
  arguments[0] = this;
  callback.apply(null, arguments);
  return this;
}

// node_modules/d3-selection/src/selection/nodes.js
function nodes_default() {
  return Array.from(this);
}

// node_modules/d3-selection/src/selection/node.js
function node_default() {
  for (var groups2 = this._groups, j = 0, m = groups2.length;j < m; ++j) {
    for (var group2 = groups2[j], i = 0, n = group2.length;i < n; ++i) {
      var node = group2[i];
      if (node)
        return node;
    }
  }
  return null;
}

// node_modules/d3-selection/src/selection/size.js
function size_default() {
  let size = 0;
  for (const node of this)
    ++size;
  return size;
}

// node_modules/d3-selection/src/selection/empty.js
function empty_default() {
  return !this.node();
}

// node_modules/d3-selection/src/selection/each.js
function each_default(callback) {
  for (var groups2 = this._groups, j = 0, m = groups2.length;j < m; ++j) {
    for (var group2 = groups2[j], i = 0, n = group2.length, node;i < n; ++i) {
      if (node = group2[i])
        callback.call(node, node.__data__, i, group2);
    }
  }
  return this;
}

// node_modules/d3-selection/src/selection/attr.js
function attrRemove(name) {
  return function() {
    this.removeAttribute(name);
  };
}
function attrRemoveNS(fullname) {
  return function() {
    this.removeAttributeNS(fullname.space, fullname.local);
  };
}
function attrConstant(name, value) {
  return function() {
    this.setAttribute(name, value);
  };
}
function attrConstantNS(fullname, value) {
  return function() {
    this.setAttributeNS(fullname.space, fullname.local, value);
  };
}
function attrFunction(name, value) {
  return function() {
    var v = value.apply(this, arguments);
    if (v == null)
      this.removeAttribute(name);
    else
      this.setAttribute(name, v);
  };
}
function attrFunctionNS(fullname, value) {
  return function() {
    var v = value.apply(this, arguments);
    if (v == null)
      this.removeAttributeNS(fullname.space, fullname.local);
    else
      this.setAttributeNS(fullname.space, fullname.local, v);
  };
}
function attr_default(name, value) {
  var fullname = namespace_default(name);
  if (arguments.length < 2) {
    var node = this.node();
    return fullname.local ? node.getAttributeNS(fullname.space, fullname.local) : node.getAttribute(fullname);
  }
  return this.each((value == null ? fullname.local ? attrRemoveNS : attrRemove : typeof value === "function" ? fullname.local ? attrFunctionNS : attrFunction : fullname.local ? attrConstantNS : attrConstant)(fullname, value));
}

// node_modules/d3-selection/src/window.js
function window_default(node) {
  return node.ownerDocument && node.ownerDocument.defaultView || node.document && node || node.defaultView;
}

// node_modules/d3-selection/src/selection/style.js
function styleRemove(name) {
  return function() {
    this.style.removeProperty(name);
  };
}
function styleConstant(name, value, priority) {
  return function() {
    this.style.setProperty(name, value, priority);
  };
}
function styleFunction(name, value, priority) {
  return function() {
    var v = value.apply(this, arguments);
    if (v == null)
      this.style.removeProperty(name);
    else
      this.style.setProperty(name, v, priority);
  };
}
function style_default(name, value, priority) {
  return arguments.length > 1 ? this.each((value == null ? styleRemove : typeof value === "function" ? styleFunction : styleConstant)(name, value, priority == null ? "" : priority)) : styleValue(this.node(), name);
}
function styleValue(node, name) {
  return node.style.getPropertyValue(name) || window_default(node).getComputedStyle(node, null).getPropertyValue(name);
}

// node_modules/d3-selection/src/selection/property.js
function propertyRemove(name) {
  return function() {
    delete this[name];
  };
}
function propertyConstant(name, value) {
  return function() {
    this[name] = value;
  };
}
function propertyFunction(name, value) {
  return function() {
    var v = value.apply(this, arguments);
    if (v == null)
      delete this[name];
    else
      this[name] = v;
  };
}
function property_default(name, value) {
  return arguments.length > 1 ? this.each((value == null ? propertyRemove : typeof value === "function" ? propertyFunction : propertyConstant)(name, value)) : this.node()[name];
}

// node_modules/d3-selection/src/selection/classed.js
function classArray(string) {
  return string.trim().split(/^|\s+/);
}
function classList(node) {
  return node.classList || new ClassList(node);
}
function ClassList(node) {
  this._node = node;
  this._names = classArray(node.getAttribute("class") || "");
}
ClassList.prototype = {
  add: function(name) {
    var i = this._names.indexOf(name);
    if (i < 0) {
      this._names.push(name);
      this._node.setAttribute("class", this._names.join(" "));
    }
  },
  remove: function(name) {
    var i = this._names.indexOf(name);
    if (i >= 0) {
      this._names.splice(i, 1);
      this._node.setAttribute("class", this._names.join(" "));
    }
  },
  contains: function(name) {
    return this._names.indexOf(name) >= 0;
  }
};
function classedAdd(node, names) {
  var list = classList(node), i = -1, n = names.length;
  while (++i < n)
    list.add(names[i]);
}
function classedRemove(node, names) {
  var list = classList(node), i = -1, n = names.length;
  while (++i < n)
    list.remove(names[i]);
}
function classedTrue(names) {
  return function() {
    classedAdd(this, names);
  };
}
function classedFalse(names) {
  return function() {
    classedRemove(this, names);
  };
}
function classedFunction(names, value) {
  return function() {
    (value.apply(this, arguments) ? classedAdd : classedRemove)(this, names);
  };
}
function classed_default(name, value) {
  var names = classArray(name + "");
  if (arguments.length < 2) {
    var list = classList(this.node()), i = -1, n = names.length;
    while (++i < n)
      if (!list.contains(names[i]))
        return false;
    return true;
  }
  return this.each((typeof value === "function" ? classedFunction : value ? classedTrue : classedFalse)(names, value));
}

// node_modules/d3-selection/src/selection/text.js
function textRemove() {
  this.textContent = "";
}
function textConstant(value) {
  return function() {
    this.textContent = value;
  };
}
function textFunction(value) {
  return function() {
    var v = value.apply(this, arguments);
    this.textContent = v == null ? "" : v;
  };
}
function text_default(value) {
  return arguments.length ? this.each(value == null ? textRemove : (typeof value === "function" ? textFunction : textConstant)(value)) : this.node().textContent;
}

// node_modules/d3-selection/src/selection/html.js
function htmlRemove() {
  this.innerHTML = "";
}
function htmlConstant(value) {
  return function() {
    this.innerHTML = value;
  };
}
function htmlFunction(value) {
  return function() {
    var v = value.apply(this, arguments);
    this.innerHTML = v == null ? "" : v;
  };
}
function html_default(value) {
  return arguments.length ? this.each(value == null ? htmlRemove : (typeof value === "function" ? htmlFunction : htmlConstant)(value)) : this.node().innerHTML;
}

// node_modules/d3-selection/src/selection/raise.js
function raise() {
  if (this.nextSibling)
    this.parentNode.appendChild(this);
}
function raise_default() {
  return this.each(raise);
}

// node_modules/d3-selection/src/selection/lower.js
function lower() {
  if (this.previousSibling)
    this.parentNode.insertBefore(this, this.parentNode.firstChild);
}
function lower_default() {
  return this.each(lower);
}

// node_modules/d3-selection/src/selection/append.js
function append_default(name) {
  var create = typeof name === "function" ? name : creator_default(name);
  return this.select(function() {
    return this.appendChild(create.apply(this, arguments));
  });
}

// node_modules/d3-selection/src/selection/insert.js
function constantNull() {
  return null;
}
function insert_default(name, before) {
  var create = typeof name === "function" ? name : creator_default(name), select = before == null ? constantNull : typeof before === "function" ? before : selector_default(before);
  return this.select(function() {
    return this.insertBefore(create.apply(this, arguments), select.apply(this, arguments) || null);
  });
}

// node_modules/d3-selection/src/selection/remove.js
function remove() {
  var parent = this.parentNode;
  if (parent)
    parent.removeChild(this);
}
function remove_default() {
  return this.each(remove);
}

// node_modules/d3-selection/src/selection/clone.js
function selection_cloneShallow() {
  var clone = this.cloneNode(false), parent = this.parentNode;
  return parent ? parent.insertBefore(clone, this.nextSibling) : clone;
}
function selection_cloneDeep() {
  var clone = this.cloneNode(true), parent = this.parentNode;
  return parent ? parent.insertBefore(clone, this.nextSibling) : clone;
}
function clone_default(deep) {
  return this.select(deep ? selection_cloneDeep : selection_cloneShallow);
}

// node_modules/d3-selection/src/selection/datum.js
function datum_default(value) {
  return arguments.length ? this.property("__data__", value) : this.node().__data__;
}

// node_modules/d3-selection/src/selection/on.js
function contextListener(listener) {
  return function(event) {
    listener.call(this, event, this.__data__);
  };
}
function parseTypenames2(typenames) {
  return typenames.trim().split(/^|\s+/).map(function(t) {
    var name = "", i = t.indexOf(".");
    if (i >= 0)
      name = t.slice(i + 1), t = t.slice(0, i);
    return { type: t, name };
  });
}
function onRemove(typename) {
  return function() {
    var on = this.__on;
    if (!on)
      return;
    for (var j = 0, i = -1, m = on.length, o;j < m; ++j) {
      if (o = on[j], (!typename.type || o.type === typename.type) && o.name === typename.name) {
        this.removeEventListener(o.type, o.listener, o.options);
      } else {
        on[++i] = o;
      }
    }
    if (++i)
      on.length = i;
    else
      delete this.__on;
  };
}
function onAdd(typename, value, options) {
  return function() {
    var on = this.__on, o, listener = contextListener(value);
    if (on)
      for (var j = 0, m = on.length;j < m; ++j) {
        if ((o = on[j]).type === typename.type && o.name === typename.name) {
          this.removeEventListener(o.type, o.listener, o.options);
          this.addEventListener(o.type, o.listener = listener, o.options = options);
          o.value = value;
          return;
        }
      }
    this.addEventListener(typename.type, listener, options);
    o = { type: typename.type, name: typename.name, value, listener, options };
    if (!on)
      this.__on = [o];
    else
      on.push(o);
  };
}
function on_default(typename, value, options) {
  var typenames = parseTypenames2(typename + ""), i, n = typenames.length, t;
  if (arguments.length < 2) {
    var on = this.node().__on;
    if (on)
      for (var j = 0, m = on.length, o;j < m; ++j) {
        for (i = 0, o = on[j];i < n; ++i) {
          if ((t = typenames[i]).type === o.type && t.name === o.name) {
            return o.value;
          }
        }
      }
    return;
  }
  on = value ? onAdd : onRemove;
  for (i = 0;i < n; ++i)
    this.each(on(typenames[i], value, options));
  return this;
}

// node_modules/d3-selection/src/selection/dispatch.js
function dispatchEvent(node, type, params) {
  var window2 = window_default(node), event = window2.CustomEvent;
  if (typeof event === "function") {
    event = new event(type, params);
  } else {
    event = window2.document.createEvent("Event");
    if (params)
      event.initEvent(type, params.bubbles, params.cancelable), event.detail = params.detail;
    else
      event.initEvent(type, false, false);
  }
  node.dispatchEvent(event);
}
function dispatchConstant(type, params) {
  return function() {
    return dispatchEvent(this, type, params);
  };
}
function dispatchFunction(type, params) {
  return function() {
    return dispatchEvent(this, type, params.apply(this, arguments));
  };
}
function dispatch_default2(type, params) {
  return this.each((typeof params === "function" ? dispatchFunction : dispatchConstant)(type, params));
}

// node_modules/d3-selection/src/selection/iterator.js
function* iterator_default() {
  for (var groups2 = this._groups, j = 0, m = groups2.length;j < m; ++j) {
    for (var group2 = groups2[j], i = 0, n = group2.length, node;i < n; ++i) {
      if (node = group2[i])
        yield node;
    }
  }
}

// node_modules/d3-selection/src/selection/index.js
var root = [null];
function Selection(groups2, parents) {
  this._groups = groups2;
  this._parents = parents;
}
function selection() {
  return new Selection([[document.documentElement]], root);
}
function selection_selection() {
  return this;
}
Selection.prototype = selection.prototype = {
  constructor: Selection,
  select: select_default,
  selectAll: selectAll_default,
  selectChild: selectChild_default,
  selectChildren: selectChildren_default,
  filter: filter_default,
  data: data_default,
  enter: enter_default,
  exit: exit_default,
  join: join_default,
  merge: merge_default,
  selection: selection_selection,
  order: order_default,
  sort: sort_default,
  call: call_default,
  nodes: nodes_default,
  node: node_default,
  size: size_default,
  empty: empty_default,
  each: each_default,
  attr: attr_default,
  style: style_default,
  property: property_default,
  classed: classed_default,
  text: text_default,
  html: html_default,
  raise: raise_default,
  lower: lower_default,
  append: append_default,
  insert: insert_default,
  remove: remove_default,
  clone: clone_default,
  datum: datum_default,
  on: on_default,
  dispatch: dispatch_default2,
  [Symbol.iterator]: iterator_default
};
var selection_default = selection;

// node_modules/d3-selection/src/select.js
function select_default2(selector) {
  return typeof selector === "string" ? new Selection([[document.querySelector(selector)]], [document.documentElement]) : new Selection([[selector]], root);
}

// node_modules/d3-selection/src/create.js
function create_default(name) {
  return select_default2(creator_default(name).call(document.documentElement));
}
// node_modules/d3-selection/src/local.js
var nextId = 0;
function local() {
  return new Local;
}
function Local() {
  this._ = "@" + (++nextId).toString(36);
}
Local.prototype = local.prototype = {
  constructor: Local,
  get: function(node) {
    var id = this._;
    while (!(id in node))
      if (!(node = node.parentNode))
        return;
    return node[id];
  },
  set: function(node, value) {
    return node[this._] = value;
  },
  remove: function(node) {
    return this._ in node && delete node[this._];
  },
  toString: function() {
    return this._;
  }
};
// node_modules/d3-selection/src/sourceEvent.js
function sourceEvent_default(event) {
  let sourceEvent;
  while (sourceEvent = event.sourceEvent)
    event = sourceEvent;
  return event;
}

// node_modules/d3-selection/src/pointer.js
function pointer_default(event, node) {
  event = sourceEvent_default(event);
  if (node === undefined)
    node = event.currentTarget;
  if (node) {
    var svg = node.ownerSVGElement || node;
    if (svg.createSVGPoint) {
      var point = svg.createSVGPoint();
      point.x = event.clientX, point.y = event.clientY;
      point = point.matrixTransform(node.getScreenCTM().inverse());
      return [point.x, point.y];
    }
    if (node.getBoundingClientRect) {
      var rect = node.getBoundingClientRect();
      return [event.clientX - rect.left - node.clientLeft, event.clientY - rect.top - node.clientTop];
    }
  }
  return [event.pageX, event.pageY];
}
// node_modules/d3-selection/src/pointers.js
function pointers_default(events, node) {
  if (events.target) {
    events = sourceEvent_default(events);
    if (node === undefined)
      node = events.currentTarget;
    events = events.touches || [events];
  }
  return Array.from(events, (event) => pointer_default(event, node));
}
// node_modules/d3-selection/src/selectAll.js
function selectAll_default2(selector) {
  return typeof selector === "string" ? new Selection([document.querySelectorAll(selector)], [document.documentElement]) : new Selection([array2(selector)], root);
}
// node_modules/d3-drag/src/noevent.js
var nonpassive = { passive: false };
var nonpassivecapture = { capture: true, passive: false };
function nopropagation(event) {
  event.stopImmediatePropagation();
}
function noevent_default(event) {
  event.preventDefault();
  event.stopImmediatePropagation();
}

// node_modules/d3-drag/src/nodrag.js
function nodrag_default(view) {
  var root2 = view.document.documentElement, selection2 = select_default2(view).on("dragstart.drag", noevent_default, nonpassivecapture);
  if ("onselectstart" in root2) {
    selection2.on("selectstart.drag", noevent_default, nonpassivecapture);
  } else {
    root2.__noselect = root2.style.MozUserSelect;
    root2.style.MozUserSelect = "none";
  }
}
function yesdrag(view, noclick) {
  var root2 = view.document.documentElement, selection2 = select_default2(view).on("dragstart.drag", null);
  if (noclick) {
    selection2.on("click.drag", noevent_default, nonpassivecapture);
    setTimeout(function() {
      selection2.on("click.drag", null);
    }, 0);
  }
  if ("onselectstart" in root2) {
    selection2.on("selectstart.drag", null);
  } else {
    root2.style.MozUserSelect = root2.__noselect;
    delete root2.__noselect;
  }
}

// node_modules/d3-drag/src/constant.js
var constant_default2 = (x) => () => x;

// node_modules/d3-drag/src/event.js
function DragEvent(type, {
  sourceEvent,
  subject,
  target,
  identifier,
  active,
  x,
  y,
  dx,
  dy,
  dispatch: dispatch2
}) {
  Object.defineProperties(this, {
    type: { value: type, enumerable: true, configurable: true },
    sourceEvent: { value: sourceEvent, enumerable: true, configurable: true },
    subject: { value: subject, enumerable: true, configurable: true },
    target: { value: target, enumerable: true, configurable: true },
    identifier: { value: identifier, enumerable: true, configurable: true },
    active: { value: active, enumerable: true, configurable: true },
    x: { value: x, enumerable: true, configurable: true },
    y: { value: y, enumerable: true, configurable: true },
    dx: { value: dx, enumerable: true, configurable: true },
    dy: { value: dy, enumerable: true, configurable: true },
    _: { value: dispatch2 }
  });
}
DragEvent.prototype.on = function() {
  var value = this._.on.apply(this._, arguments);
  return value === this._ ? this : value;
};

// node_modules/d3-drag/src/drag.js
function defaultFilter(event) {
  return !event.ctrlKey && !event.button;
}
function defaultContainer() {
  return this.parentNode;
}
function defaultSubject(event, d) {
  return d == null ? { x: event.x, y: event.y } : d;
}
function defaultTouchable() {
  return navigator.maxTouchPoints || "ontouchstart" in this;
}
function drag_default() {
  var filter3 = defaultFilter, container = defaultContainer, subject = defaultSubject, touchable = defaultTouchable, gestures = {}, listeners = dispatch_default("start", "drag", "end"), active = 0, mousedownx, mousedowny, mousemoving, touchending, clickDistance2 = 0;
  function drag(selection2) {
    selection2.on("mousedown.drag", mousedowned).filter(touchable).on("touchstart.drag", touchstarted).on("touchmove.drag", touchmoved, nonpassive).on("touchend.drag touchcancel.drag", touchended).style("touch-action", "none").style("-webkit-tap-highlight-color", "rgba(0,0,0,0)");
  }
  function mousedowned(event, d) {
    if (touchending || !filter3.call(this, event, d))
      return;
    var gesture = beforestart(this, container.call(this, event, d), event, d, "mouse");
    if (!gesture)
      return;
    select_default2(event.view).on("mousemove.drag", mousemoved, nonpassivecapture).on("mouseup.drag", mouseupped, nonpassivecapture);
    nodrag_default(event.view);
    nopropagation(event);
    mousemoving = false;
    mousedownx = event.clientX;
    mousedowny = event.clientY;
    gesture("start", event);
  }
  function mousemoved(event) {
    noevent_default(event);
    if (!mousemoving) {
      var dx = event.clientX - mousedownx, dy = event.clientY - mousedowny;
      mousemoving = dx * dx + dy * dy > clickDistance2;
    }
    gestures.mouse("drag", event);
  }
  function mouseupped(event) {
    select_default2(event.view).on("mousemove.drag mouseup.drag", null);
    yesdrag(event.view, mousemoving);
    noevent_default(event);
    gestures.mouse("end", event);
  }
  function touchstarted(event, d) {
    if (!filter3.call(this, event, d))
      return;
    var touches = event.changedTouches, c = container.call(this, event, d), n = touches.length, i, gesture;
    for (i = 0;i < n; ++i) {
      if (gesture = beforestart(this, c, event, d, touches[i].identifier, touches[i])) {
        nopropagation(event);
        gesture("start", event, touches[i]);
      }
    }
  }
  function touchmoved(event) {
    var touches = event.changedTouches, n = touches.length, i, gesture;
    for (i = 0;i < n; ++i) {
      if (gesture = gestures[touches[i].identifier]) {
        noevent_default(event);
        gesture("drag", event, touches[i]);
      }
    }
  }
  function touchended(event) {
    var touches = event.changedTouches, n = touches.length, i, gesture;
    if (touchending)
      clearTimeout(touchending);
    touchending = setTimeout(function() {
      touchending = null;
    }, 500);
    for (i = 0;i < n; ++i) {
      if (gesture = gestures[touches[i].identifier]) {
        nopropagation(event);
        gesture("end", event, touches[i]);
      }
    }
  }
  function beforestart(that, container2, event, d, identifier, touch) {
    var dispatch2 = listeners.copy(), p = pointer_default(touch || event, container2), dx, dy, s;
    if ((s = subject.call(that, new DragEvent("beforestart", {
      sourceEvent: event,
      target: drag,
      identifier,
      active,
      x: p[0],
      y: p[1],
      dx: 0,
      dy: 0,
      dispatch: dispatch2
    }), d)) == null)
      return;
    dx = s.x - p[0] || 0;
    dy = s.y - p[1] || 0;
    return function gesture(type, event2, touch2) {
      var p0 = p, n;
      switch (type) {
        case "start":
          gestures[identifier] = gesture, n = active++;
          break;
        case "end":
          delete gestures[identifier], --active;
        case "drag":
          p = pointer_default(touch2 || event2, container2), n = active;
          break;
      }
      dispatch2.call(type, that, new DragEvent(type, {
        sourceEvent: event2,
        subject: s,
        target: drag,
        identifier,
        active: n,
        x: p[0] + dx,
        y: p[1] + dy,
        dx: p[0] - p0[0],
        dy: p[1] - p0[1],
        dispatch: dispatch2
      }), d);
    };
  }
  drag.filter = function(_) {
    return arguments.length ? (filter3 = typeof _ === "function" ? _ : constant_default2(!!_), drag) : filter3;
  };
  drag.container = function(_) {
    return arguments.length ? (container = typeof _ === "function" ? _ : constant_default2(_), drag) : container;
  };
  drag.subject = function(_) {
    return arguments.length ? (subject = typeof _ === "function" ? _ : constant_default2(_), drag) : subject;
  };
  drag.touchable = function(_) {
    return arguments.length ? (touchable = typeof _ === "function" ? _ : constant_default2(!!_), drag) : touchable;
  };
  drag.on = function() {
    var value = listeners.on.apply(listeners, arguments);
    return value === listeners ? drag : value;
  };
  drag.clickDistance = function(_) {
    return arguments.length ? (clickDistance2 = (_ = +_) * _, drag) : Math.sqrt(clickDistance2);
  };
  return drag;
}
// node_modules/d3-color/src/define.js
function define_default(constructor, factory, prototype) {
  constructor.prototype = factory.prototype = prototype;
  prototype.constructor = constructor;
}
function extend(parent, definition) {
  var prototype = Object.create(parent.prototype);
  for (var key in definition)
    prototype[key] = definition[key];
  return prototype;
}

// node_modules/d3-color/src/color.js
function Color() {}
var darker = 0.7;
var brighter = 1 / darker;
var reI = "\\s*([+-]?\\d+)\\s*";
var reN = "\\s*([+-]?(?:\\d*\\.)?\\d+(?:[eE][+-]?\\d+)?)\\s*";
var reP = "\\s*([+-]?(?:\\d*\\.)?\\d+(?:[eE][+-]?\\d+)?)%\\s*";
var reHex = /^#([0-9a-f]{3,8})$/;
var reRgbInteger = new RegExp(`^rgb\\(${reI},${reI},${reI}\\)$`);
var reRgbPercent = new RegExp(`^rgb\\(${reP},${reP},${reP}\\)$`);
var reRgbaInteger = new RegExp(`^rgba\\(${reI},${reI},${reI},${reN}\\)$`);
var reRgbaPercent = new RegExp(`^rgba\\(${reP},${reP},${reP},${reN}\\)$`);
var reHslPercent = new RegExp(`^hsl\\(${reN},${reP},${reP}\\)$`);
var reHslaPercent = new RegExp(`^hsla\\(${reN},${reP},${reP},${reN}\\)$`);
var named = {
  aliceblue: 15792383,
  antiquewhite: 16444375,
  aqua: 65535,
  aquamarine: 8388564,
  azure: 15794175,
  beige: 16119260,
  bisque: 16770244,
  black: 0,
  blanchedalmond: 16772045,
  blue: 255,
  blueviolet: 9055202,
  brown: 10824234,
  burlywood: 14596231,
  cadetblue: 6266528,
  chartreuse: 8388352,
  chocolate: 13789470,
  coral: 16744272,
  cornflowerblue: 6591981,
  cornsilk: 16775388,
  crimson: 14423100,
  cyan: 65535,
  darkblue: 139,
  darkcyan: 35723,
  darkgoldenrod: 12092939,
  darkgray: 11119017,
  darkgreen: 25600,
  darkgrey: 11119017,
  darkkhaki: 12433259,
  darkmagenta: 9109643,
  darkolivegreen: 5597999,
  darkorange: 16747520,
  darkorchid: 10040012,
  darkred: 9109504,
  darksalmon: 15308410,
  darkseagreen: 9419919,
  darkslateblue: 4734347,
  darkslategray: 3100495,
  darkslategrey: 3100495,
  darkturquoise: 52945,
  darkviolet: 9699539,
  deeppink: 16716947,
  deepskyblue: 49151,
  dimgray: 6908265,
  dimgrey: 6908265,
  dodgerblue: 2003199,
  firebrick: 11674146,
  floralwhite: 16775920,
  forestgreen: 2263842,
  fuchsia: 16711935,
  gainsboro: 14474460,
  ghostwhite: 16316671,
  gold: 16766720,
  goldenrod: 14329120,
  gray: 8421504,
  green: 32768,
  greenyellow: 11403055,
  grey: 8421504,
  honeydew: 15794160,
  hotpink: 16738740,
  indianred: 13458524,
  indigo: 4915330,
  ivory: 16777200,
  khaki: 15787660,
  lavender: 15132410,
  lavenderblush: 16773365,
  lawngreen: 8190976,
  lemonchiffon: 16775885,
  lightblue: 11393254,
  lightcoral: 15761536,
  lightcyan: 14745599,
  lightgoldenrodyellow: 16448210,
  lightgray: 13882323,
  lightgreen: 9498256,
  lightgrey: 13882323,
  lightpink: 16758465,
  lightsalmon: 16752762,
  lightseagreen: 2142890,
  lightskyblue: 8900346,
  lightslategray: 7833753,
  lightslategrey: 7833753,
  lightsteelblue: 11584734,
  lightyellow: 16777184,
  lime: 65280,
  limegreen: 3329330,
  linen: 16445670,
  magenta: 16711935,
  maroon: 8388608,
  mediumaquamarine: 6737322,
  mediumblue: 205,
  mediumorchid: 12211667,
  mediumpurple: 9662683,
  mediumseagreen: 3978097,
  mediumslateblue: 8087790,
  mediumspringgreen: 64154,
  mediumturquoise: 4772300,
  mediumvioletred: 13047173,
  midnightblue: 1644912,
  mintcream: 16121850,
  mistyrose: 16770273,
  moccasin: 16770229,
  navajowhite: 16768685,
  navy: 128,
  oldlace: 16643558,
  olive: 8421376,
  olivedrab: 7048739,
  orange: 16753920,
  orangered: 16729344,
  orchid: 14315734,
  palegoldenrod: 15657130,
  palegreen: 10025880,
  paleturquoise: 11529966,
  palevioletred: 14381203,
  papayawhip: 16773077,
  peachpuff: 16767673,
  peru: 13468991,
  pink: 16761035,
  plum: 14524637,
  powderblue: 11591910,
  purple: 8388736,
  rebeccapurple: 6697881,
  red: 16711680,
  rosybrown: 12357519,
  royalblue: 4286945,
  saddlebrown: 9127187,
  salmon: 16416882,
  sandybrown: 16032864,
  seagreen: 3050327,
  seashell: 16774638,
  sienna: 10506797,
  silver: 12632256,
  skyblue: 8900331,
  slateblue: 6970061,
  slategray: 7372944,
  slategrey: 7372944,
  snow: 16775930,
  springgreen: 65407,
  steelblue: 4620980,
  tan: 13808780,
  teal: 32896,
  thistle: 14204888,
  tomato: 16737095,
  turquoise: 4251856,
  violet: 15631086,
  wheat: 16113331,
  white: 16777215,
  whitesmoke: 16119285,
  yellow: 16776960,
  yellowgreen: 10145074
};
define_default(Color, color, {
  copy(channels) {
    return Object.assign(new this.constructor, this, channels);
  },
  displayable() {
    return this.rgb().displayable();
  },
  hex: color_formatHex,
  formatHex: color_formatHex,
  formatHex8: color_formatHex8,
  formatHsl: color_formatHsl,
  formatRgb: color_formatRgb,
  toString: color_formatRgb
});
function color_formatHex() {
  return this.rgb().formatHex();
}
function color_formatHex8() {
  return this.rgb().formatHex8();
}
function color_formatHsl() {
  return hslConvert(this).formatHsl();
}
function color_formatRgb() {
  return this.rgb().formatRgb();
}
function color(format) {
  var m, l;
  format = (format + "").trim().toLowerCase();
  return (m = reHex.exec(format)) ? (l = m[1].length, m = parseInt(m[1], 16), l === 6 ? rgbn(m) : l === 3 ? new Rgb(m >> 8 & 15 | m >> 4 & 240, m >> 4 & 15 | m & 240, (m & 15) << 4 | m & 15, 1) : l === 8 ? rgba(m >> 24 & 255, m >> 16 & 255, m >> 8 & 255, (m & 255) / 255) : l === 4 ? rgba(m >> 12 & 15 | m >> 8 & 240, m >> 8 & 15 | m >> 4 & 240, m >> 4 & 15 | m & 240, ((m & 15) << 4 | m & 15) / 255) : null) : (m = reRgbInteger.exec(format)) ? new Rgb(m[1], m[2], m[3], 1) : (m = reRgbPercent.exec(format)) ? new Rgb(m[1] * 255 / 100, m[2] * 255 / 100, m[3] * 255 / 100, 1) : (m = reRgbaInteger.exec(format)) ? rgba(m[1], m[2], m[3], m[4]) : (m = reRgbaPercent.exec(format)) ? rgba(m[1] * 255 / 100, m[2] * 255 / 100, m[3] * 255 / 100, m[4]) : (m = reHslPercent.exec(format)) ? hsla(m[1], m[2] / 100, m[3] / 100, 1) : (m = reHslaPercent.exec(format)) ? hsla(m[1], m[2] / 100, m[3] / 100, m[4]) : named.hasOwnProperty(format) ? rgbn(named[format]) : format === "transparent" ? new Rgb(NaN, NaN, NaN, 0) : null;
}
function rgbn(n) {
  return new Rgb(n >> 16 & 255, n >> 8 & 255, n & 255, 1);
}
function rgba(r, g, b, a) {
  if (a <= 0)
    r = g = b = NaN;
  return new Rgb(r, g, b, a);
}
function rgbConvert(o) {
  if (!(o instanceof Color))
    o = color(o);
  if (!o)
    return new Rgb;
  o = o.rgb();
  return new Rgb(o.r, o.g, o.b, o.opacity);
}
function rgb(r, g, b, opacity) {
  return arguments.length === 1 ? rgbConvert(r) : new Rgb(r, g, b, opacity == null ? 1 : opacity);
}
function Rgb(r, g, b, opacity) {
  this.r = +r;
  this.g = +g;
  this.b = +b;
  this.opacity = +opacity;
}
define_default(Rgb, rgb, extend(Color, {
  brighter(k) {
    k = k == null ? brighter : Math.pow(brighter, k);
    return new Rgb(this.r * k, this.g * k, this.b * k, this.opacity);
  },
  darker(k) {
    k = k == null ? darker : Math.pow(darker, k);
    return new Rgb(this.r * k, this.g * k, this.b * k, this.opacity);
  },
  rgb() {
    return this;
  },
  clamp() {
    return new Rgb(clampi(this.r), clampi(this.g), clampi(this.b), clampa(this.opacity));
  },
  displayable() {
    return -0.5 <= this.r && this.r < 255.5 && (-0.5 <= this.g && this.g < 255.5) && (-0.5 <= this.b && this.b < 255.5) && (0 <= this.opacity && this.opacity <= 1);
  },
  hex: rgb_formatHex,
  formatHex: rgb_formatHex,
  formatHex8: rgb_formatHex8,
  formatRgb: rgb_formatRgb,
  toString: rgb_formatRgb
}));
function rgb_formatHex() {
  return `#${hex(this.r)}${hex(this.g)}${hex(this.b)}`;
}
function rgb_formatHex8() {
  return `#${hex(this.r)}${hex(this.g)}${hex(this.b)}${hex((isNaN(this.opacity) ? 1 : this.opacity) * 255)}`;
}
function rgb_formatRgb() {
  const a = clampa(this.opacity);
  return `${a === 1 ? "rgb(" : "rgba("}${clampi(this.r)}, ${clampi(this.g)}, ${clampi(this.b)}${a === 1 ? ")" : `, ${a})`}`;
}
function clampa(opacity) {
  return isNaN(opacity) ? 1 : Math.max(0, Math.min(1, opacity));
}
function clampi(value) {
  return Math.max(0, Math.min(255, Math.round(value) || 0));
}
function hex(value) {
  value = clampi(value);
  return (value < 16 ? "0" : "") + value.toString(16);
}
function hsla(h, s, l, a) {
  if (a <= 0)
    h = s = l = NaN;
  else if (l <= 0 || l >= 1)
    h = s = NaN;
  else if (s <= 0)
    h = NaN;
  return new Hsl(h, s, l, a);
}
function hslConvert(o) {
  if (o instanceof Hsl)
    return new Hsl(o.h, o.s, o.l, o.opacity);
  if (!(o instanceof Color))
    o = color(o);
  if (!o)
    return new Hsl;
  if (o instanceof Hsl)
    return o;
  o = o.rgb();
  var r = o.r / 255, g = o.g / 255, b = o.b / 255, min2 = Math.min(r, g, b), max2 = Math.max(r, g, b), h = NaN, s = max2 - min2, l = (max2 + min2) / 2;
  if (s) {
    if (r === max2)
      h = (g - b) / s + (g < b) * 6;
    else if (g === max2)
      h = (b - r) / s + 2;
    else
      h = (r - g) / s + 4;
    s /= l < 0.5 ? max2 + min2 : 2 - max2 - min2;
    h *= 60;
  } else {
    s = l > 0 && l < 1 ? 0 : h;
  }
  return new Hsl(h, s, l, o.opacity);
}
function hsl(h, s, l, opacity) {
  return arguments.length === 1 ? hslConvert(h) : new Hsl(h, s, l, opacity == null ? 1 : opacity);
}
function Hsl(h, s, l, opacity) {
  this.h = +h;
  this.s = +s;
  this.l = +l;
  this.opacity = +opacity;
}
define_default(Hsl, hsl, extend(Color, {
  brighter(k) {
    k = k == null ? brighter : Math.pow(brighter, k);
    return new Hsl(this.h, this.s, this.l * k, this.opacity);
  },
  darker(k) {
    k = k == null ? darker : Math.pow(darker, k);
    return new Hsl(this.h, this.s, this.l * k, this.opacity);
  },
  rgb() {
    var h = this.h % 360 + (this.h < 0) * 360, s = isNaN(h) || isNaN(this.s) ? 0 : this.s, l = this.l, m2 = l + (l < 0.5 ? l : 1 - l) * s, m1 = 2 * l - m2;
    return new Rgb(hsl2rgb(h >= 240 ? h - 240 : h + 120, m1, m2), hsl2rgb(h, m1, m2), hsl2rgb(h < 120 ? h + 240 : h - 120, m1, m2), this.opacity);
  },
  clamp() {
    return new Hsl(clamph(this.h), clampt(this.s), clampt(this.l), clampa(this.opacity));
  },
  displayable() {
    return (0 <= this.s && this.s <= 1 || isNaN(this.s)) && (0 <= this.l && this.l <= 1) && (0 <= this.opacity && this.opacity <= 1);
  },
  formatHsl() {
    const a = clampa(this.opacity);
    return `${a === 1 ? "hsl(" : "hsla("}${clamph(this.h)}, ${clampt(this.s) * 100}%, ${clampt(this.l) * 100}%${a === 1 ? ")" : `, ${a})`}`;
  }
}));
function clamph(value) {
  value = (value || 0) % 360;
  return value < 0 ? value + 360 : value;
}
function clampt(value) {
  return Math.max(0, Math.min(1, value || 0));
}
function hsl2rgb(h, m1, m2) {
  return (h < 60 ? m1 + (m2 - m1) * h / 60 : h < 180 ? m2 : h < 240 ? m1 + (m2 - m1) * (240 - h) / 60 : m1) * 255;
}
// node_modules/d3-color/src/math.js
var radians = Math.PI / 180;
var degrees = 180 / Math.PI;

// node_modules/d3-color/src/lab.js
var K = 18;
var Xn = 0.96422;
var Yn = 1;
var Zn = 0.82521;
var t0 = 4 / 29;
var t1 = 6 / 29;
var t2 = 3 * t1 * t1;
var t3 = t1 * t1 * t1;
function labConvert(o) {
  if (o instanceof Lab)
    return new Lab(o.l, o.a, o.b, o.opacity);
  if (o instanceof Hcl)
    return hcl2lab(o);
  if (!(o instanceof Rgb))
    o = rgbConvert(o);
  var r = rgb2lrgb(o.r), g = rgb2lrgb(o.g), b = rgb2lrgb(o.b), y = xyz2lab((0.2225045 * r + 0.7168786 * g + 0.0606169 * b) / Yn), x, z;
  if (r === g && g === b)
    x = z = y;
  else {
    x = xyz2lab((0.4360747 * r + 0.3850649 * g + 0.1430804 * b) / Xn);
    z = xyz2lab((0.0139322 * r + 0.0971045 * g + 0.7141733 * b) / Zn);
  }
  return new Lab(116 * y - 16, 500 * (x - y), 200 * (y - z), o.opacity);
}
function gray(l, opacity) {
  return new Lab(l, 0, 0, opacity == null ? 1 : opacity);
}
function lab(l, a, b, opacity) {
  return arguments.length === 1 ? labConvert(l) : new Lab(l, a, b, opacity == null ? 1 : opacity);
}
function Lab(l, a, b, opacity) {
  this.l = +l;
  this.a = +a;
  this.b = +b;
  this.opacity = +opacity;
}
define_default(Lab, lab, extend(Color, {
  brighter(k) {
    return new Lab(this.l + K * (k == null ? 1 : k), this.a, this.b, this.opacity);
  },
  darker(k) {
    return new Lab(this.l - K * (k == null ? 1 : k), this.a, this.b, this.opacity);
  },
  rgb() {
    var y = (this.l + 16) / 116, x = isNaN(this.a) ? y : y + this.a / 500, z = isNaN(this.b) ? y : y - this.b / 200;
    x = Xn * lab2xyz(x);
    y = Yn * lab2xyz(y);
    z = Zn * lab2xyz(z);
    return new Rgb(lrgb2rgb(3.1338561 * x - 1.6168667 * y - 0.4906146 * z), lrgb2rgb(-0.9787684 * x + 1.9161415 * y + 0.033454 * z), lrgb2rgb(0.0719453 * x - 0.2289914 * y + 1.4052427 * z), this.opacity);
  }
}));
function xyz2lab(t) {
  return t > t3 ? Math.pow(t, 1 / 3) : t / t2 + t0;
}
function lab2xyz(t) {
  return t > t1 ? t * t * t : t2 * (t - t0);
}
function lrgb2rgb(x) {
  return 255 * (x <= 0.0031308 ? 12.92 * x : 1.055 * Math.pow(x, 1 / 2.4) - 0.055);
}
function rgb2lrgb(x) {
  return (x /= 255) <= 0.04045 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4);
}
function hclConvert(o) {
  if (o instanceof Hcl)
    return new Hcl(o.h, o.c, o.l, o.opacity);
  if (!(o instanceof Lab))
    o = labConvert(o);
  if (o.a === 0 && o.b === 0)
    return new Hcl(NaN, 0 < o.l && o.l < 100 ? 0 : NaN, o.l, o.opacity);
  var h = Math.atan2(o.b, o.a) * degrees;
  return new Hcl(h < 0 ? h + 360 : h, Math.sqrt(o.a * o.a + o.b * o.b), o.l, o.opacity);
}
function lch(l, c, h, opacity) {
  return arguments.length === 1 ? hclConvert(l) : new Hcl(h, c, l, opacity == null ? 1 : opacity);
}
function hcl(h, c, l, opacity) {
  return arguments.length === 1 ? hclConvert(h) : new Hcl(h, c, l, opacity == null ? 1 : opacity);
}
function Hcl(h, c, l, opacity) {
  this.h = +h;
  this.c = +c;
  this.l = +l;
  this.opacity = +opacity;
}
function hcl2lab(o) {
  if (isNaN(o.h))
    return new Lab(o.l, 0, 0, o.opacity);
  var h = o.h * radians;
  return new Lab(o.l, Math.cos(h) * o.c, Math.sin(h) * o.c, o.opacity);
}
define_default(Hcl, hcl, extend(Color, {
  brighter(k) {
    return new Hcl(this.h, this.c, this.l + K * (k == null ? 1 : k), this.opacity);
  },
  darker(k) {
    return new Hcl(this.h, this.c, this.l - K * (k == null ? 1 : k), this.opacity);
  },
  rgb() {
    return hcl2lab(this).rgb();
  }
}));
// node_modules/d3-color/src/cubehelix.js
var A = -0.14861;
var B = 1.78277;
var C = -0.29227;
var D = -0.90649;
var E = 1.97294;
var ED = E * D;
var EB = E * B;
var BC_DA = B * C - D * A;
function cubehelixConvert(o) {
  if (o instanceof Cubehelix)
    return new Cubehelix(o.h, o.s, o.l, o.opacity);
  if (!(o instanceof Rgb))
    o = rgbConvert(o);
  var r = o.r / 255, g = o.g / 255, b = o.b / 255, l = (BC_DA * b + ED * r - EB * g) / (BC_DA + ED - EB), bl = b - l, k = (E * (g - l) - C * bl) / D, s = Math.sqrt(k * k + bl * bl) / (E * l * (1 - l)), h = s ? Math.atan2(k, bl) * degrees - 120 : NaN;
  return new Cubehelix(h < 0 ? h + 360 : h, s, l, o.opacity);
}
function cubehelix(h, s, l, opacity) {
  return arguments.length === 1 ? cubehelixConvert(h) : new Cubehelix(h, s, l, opacity == null ? 1 : opacity);
}
function Cubehelix(h, s, l, opacity) {
  this.h = +h;
  this.s = +s;
  this.l = +l;
  this.opacity = +opacity;
}
define_default(Cubehelix, cubehelix, extend(Color, {
  brighter(k) {
    k = k == null ? brighter : Math.pow(brighter, k);
    return new Cubehelix(this.h, this.s, this.l * k, this.opacity);
  },
  darker(k) {
    k = k == null ? darker : Math.pow(darker, k);
    return new Cubehelix(this.h, this.s, this.l * k, this.opacity);
  },
  rgb() {
    var h = isNaN(this.h) ? 0 : (this.h + 120) * radians, l = +this.l, a = isNaN(this.s) ? 0 : this.s * l * (1 - l), cosh = Math.cos(h), sinh = Math.sin(h);
    return new Rgb(255 * (l + a * (A * cosh + B * sinh)), 255 * (l + a * (C * cosh + D * sinh)), 255 * (l + a * (E * cosh)), this.opacity);
  }
}));
// node_modules/d3-interpolate/src/basis.js
function basis(t12, v0, v1, v2, v3) {
  var t22 = t12 * t12, t32 = t22 * t12;
  return ((1 - 3 * t12 + 3 * t22 - t32) * v0 + (4 - 6 * t22 + 3 * t32) * v1 + (1 + 3 * t12 + 3 * t22 - 3 * t32) * v2 + t32 * v3) / 6;
}
function basis_default(values) {
  var n = values.length - 1;
  return function(t) {
    var i = t <= 0 ? t = 0 : t >= 1 ? (t = 1, n - 1) : Math.floor(t * n), v1 = values[i], v2 = values[i + 1], v0 = i > 0 ? values[i - 1] : 2 * v1 - v2, v3 = i < n - 1 ? values[i + 2] : 2 * v2 - v1;
    return basis((t - i / n) * n, v0, v1, v2, v3);
  };
}

// node_modules/d3-interpolate/src/basisClosed.js
function basisClosed_default(values) {
  var n = values.length;
  return function(t) {
    var i = Math.floor(((t %= 1) < 0 ? ++t : t) * n), v0 = values[(i + n - 1) % n], v1 = values[i % n], v2 = values[(i + 1) % n], v3 = values[(i + 2) % n];
    return basis((t - i / n) * n, v0, v1, v2, v3);
  };
}

// node_modules/d3-interpolate/src/constant.js
var constant_default3 = (x) => () => x;

// node_modules/d3-interpolate/src/color.js
function linear(a, d) {
  return function(t) {
    return a + t * d;
  };
}
function exponential(a, b, y) {
  return a = Math.pow(a, y), b = Math.pow(b, y) - a, y = 1 / y, function(t) {
    return Math.pow(a + t * b, y);
  };
}
function hue(a, b) {
  var d = b - a;
  return d ? linear(a, d > 180 || d < -180 ? d - 360 * Math.round(d / 360) : d) : constant_default3(isNaN(a) ? b : a);
}
function gamma(y) {
  return (y = +y) === 1 ? nogamma : function(a, b) {
    return b - a ? exponential(a, b, y) : constant_default3(isNaN(a) ? b : a);
  };
}
function nogamma(a, b) {
  var d = b - a;
  return d ? linear(a, d) : constant_default3(isNaN(a) ? b : a);
}

// node_modules/d3-interpolate/src/rgb.js
var rgb_default = function rgbGamma(y) {
  var color2 = gamma(y);
  function rgb2(start, end) {
    var r = color2((start = rgb(start)).r, (end = rgb(end)).r), g = color2(start.g, end.g), b = color2(start.b, end.b), opacity = nogamma(start.opacity, end.opacity);
    return function(t) {
      start.r = r(t);
      start.g = g(t);
      start.b = b(t);
      start.opacity = opacity(t);
      return start + "";
    };
  }
  rgb2.gamma = rgbGamma;
  return rgb2;
}(1);
function rgbSpline(spline) {
  return function(colors) {
    var n = colors.length, r = new Array(n), g = new Array(n), b = new Array(n), i, color2;
    for (i = 0;i < n; ++i) {
      color2 = rgb(colors[i]);
      r[i] = color2.r || 0;
      g[i] = color2.g || 0;
      b[i] = color2.b || 0;
    }
    r = spline(r);
    g = spline(g);
    b = spline(b);
    color2.opacity = 1;
    return function(t) {
      color2.r = r(t);
      color2.g = g(t);
      color2.b = b(t);
      return color2 + "";
    };
  };
}
var rgbBasis = rgbSpline(basis_default);
var rgbBasisClosed = rgbSpline(basisClosed_default);

// node_modules/d3-interpolate/src/numberArray.js
function numberArray_default(a, b) {
  if (!b)
    b = [];
  var n = a ? Math.min(b.length, a.length) : 0, c = b.slice(), i;
  return function(t) {
    for (i = 0;i < n; ++i)
      c[i] = a[i] * (1 - t) + b[i] * t;
    return c;
  };
}
function isNumberArray(x) {
  return ArrayBuffer.isView(x) && !(x instanceof DataView);
}

// node_modules/d3-interpolate/src/array.js
function array_default(a, b) {
  return (isNumberArray(b) ? numberArray_default : genericArray)(a, b);
}
function genericArray(a, b) {
  var nb = b ? b.length : 0, na = a ? Math.min(nb, a.length) : 0, x = new Array(na), c = new Array(nb), i;
  for (i = 0;i < na; ++i)
    x[i] = value_default(a[i], b[i]);
  for (;i < nb; ++i)
    c[i] = b[i];
  return function(t) {
    for (i = 0;i < na; ++i)
      c[i] = x[i](t);
    return c;
  };
}

// node_modules/d3-interpolate/src/date.js
function date_default(a, b) {
  var d = new Date;
  return a = +a, b = +b, function(t) {
    return d.setTime(a * (1 - t) + b * t), d;
  };
}

// node_modules/d3-interpolate/src/number.js
function number_default(a, b) {
  return a = +a, b = +b, function(t) {
    return a * (1 - t) + b * t;
  };
}

// node_modules/d3-interpolate/src/object.js
function object_default(a, b) {
  var i = {}, c = {}, k;
  if (a === null || typeof a !== "object")
    a = {};
  if (b === null || typeof b !== "object")
    b = {};
  for (k in b) {
    if (k in a) {
      i[k] = value_default(a[k], b[k]);
    } else {
      c[k] = b[k];
    }
  }
  return function(t) {
    for (k in i)
      c[k] = i[k](t);
    return c;
  };
}

// node_modules/d3-interpolate/src/string.js
var reA = /[-+]?(?:\d+\.?\d*|\.?\d+)(?:[eE][-+]?\d+)?/g;
var reB = new RegExp(reA.source, "g");
function zero2(b) {
  return function() {
    return b;
  };
}
function one(b) {
  return function(t) {
    return b(t) + "";
  };
}
function string_default(a, b) {
  var bi = reA.lastIndex = reB.lastIndex = 0, am, bm, bs, i = -1, s = [], q = [];
  a = a + "", b = b + "";
  while ((am = reA.exec(a)) && (bm = reB.exec(b))) {
    if ((bs = bm.index) > bi) {
      bs = b.slice(bi, bs);
      if (s[i])
        s[i] += bs;
      else
        s[++i] = bs;
    }
    if ((am = am[0]) === (bm = bm[0])) {
      if (s[i])
        s[i] += bm;
      else
        s[++i] = bm;
    } else {
      s[++i] = null;
      q.push({ i, x: number_default(am, bm) });
    }
    bi = reB.lastIndex;
  }
  if (bi < b.length) {
    bs = b.slice(bi);
    if (s[i])
      s[i] += bs;
    else
      s[++i] = bs;
  }
  return s.length < 2 ? q[0] ? one(q[0].x) : zero2(b) : (b = q.length, function(t) {
    for (var i2 = 0, o;i2 < b; ++i2)
      s[(o = q[i2]).i] = o.x(t);
    return s.join("");
  });
}

// node_modules/d3-interpolate/src/value.js
function value_default(a, b) {
  var t = typeof b, c;
  return b == null || t === "boolean" ? constant_default3(b) : (t === "number" ? number_default : t === "string" ? (c = color(b)) ? (b = c, rgb_default) : string_default : b instanceof color ? rgb_default : b instanceof Date ? date_default : isNumberArray(b) ? numberArray_default : Array.isArray(b) ? genericArray : typeof b.valueOf !== "function" && typeof b.toString !== "function" || isNaN(b) ? object_default : number_default)(a, b);
}
// node_modules/d3-interpolate/src/discrete.js
function discrete_default(range2) {
  var n = range2.length;
  return function(t) {
    return range2[Math.max(0, Math.min(n - 1, Math.floor(t * n)))];
  };
}
// node_modules/d3-interpolate/src/hue.js
function hue_default(a, b) {
  var i = hue(+a, +b);
  return function(t) {
    var x = i(t);
    return x - 360 * Math.floor(x / 360);
  };
}
// node_modules/d3-interpolate/src/round.js
function round_default(a, b) {
  return a = +a, b = +b, function(t) {
    return Math.round(a * (1 - t) + b * t);
  };
}
// node_modules/d3-interpolate/src/transform/decompose.js
var degrees2 = 180 / Math.PI;
var identity2 = {
  translateX: 0,
  translateY: 0,
  rotate: 0,
  skewX: 0,
  scaleX: 1,
  scaleY: 1
};
function decompose_default(a, b, c, d, e, f) {
  var scaleX, scaleY, skewX;
  if (scaleX = Math.sqrt(a * a + b * b))
    a /= scaleX, b /= scaleX;
  if (skewX = a * c + b * d)
    c -= a * skewX, d -= b * skewX;
  if (scaleY = Math.sqrt(c * c + d * d))
    c /= scaleY, d /= scaleY, skewX /= scaleY;
  if (a * d < b * c)
    a = -a, b = -b, skewX = -skewX, scaleX = -scaleX;
  return {
    translateX: e,
    translateY: f,
    rotate: Math.atan2(b, a) * degrees2,
    skewX: Math.atan(skewX) * degrees2,
    scaleX,
    scaleY
  };
}

// node_modules/d3-interpolate/src/transform/parse.js
var svgNode;
function parseCss(value) {
  const m = new (typeof DOMMatrix === "function" ? DOMMatrix : WebKitCSSMatrix)(value + "");
  return m.isIdentity ? identity2 : decompose_default(m.a, m.b, m.c, m.d, m.e, m.f);
}
function parseSvg(value) {
  if (value == null)
    return identity2;
  if (!svgNode)
    svgNode = document.createElementNS("http://www.w3.org/2000/svg", "g");
  svgNode.setAttribute("transform", value);
  if (!(value = svgNode.transform.baseVal.consolidate()))
    return identity2;
  value = value.matrix;
  return decompose_default(value.a, value.b, value.c, value.d, value.e, value.f);
}

// node_modules/d3-interpolate/src/transform/index.js
function interpolateTransform(parse, pxComma, pxParen, degParen) {
  function pop(s) {
    return s.length ? s.pop() + " " : "";
  }
  function translate(xa, ya, xb, yb, s, q) {
    if (xa !== xb || ya !== yb) {
      var i = s.push("translate(", null, pxComma, null, pxParen);
      q.push({ i: i - 4, x: number_default(xa, xb) }, { i: i - 2, x: number_default(ya, yb) });
    } else if (xb || yb) {
      s.push("translate(" + xb + pxComma + yb + pxParen);
    }
  }
  function rotate(a, b, s, q) {
    if (a !== b) {
      if (a - b > 180)
        b += 360;
      else if (b - a > 180)
        a += 360;
      q.push({ i: s.push(pop(s) + "rotate(", null, degParen) - 2, x: number_default(a, b) });
    } else if (b) {
      s.push(pop(s) + "rotate(" + b + degParen);
    }
  }
  function skewX(a, b, s, q) {
    if (a !== b) {
      q.push({ i: s.push(pop(s) + "skewX(", null, degParen) - 2, x: number_default(a, b) });
    } else if (b) {
      s.push(pop(s) + "skewX(" + b + degParen);
    }
  }
  function scale(xa, ya, xb, yb, s, q) {
    if (xa !== xb || ya !== yb) {
      var i = s.push(pop(s) + "scale(", null, ",", null, ")");
      q.push({ i: i - 4, x: number_default(xa, xb) }, { i: i - 2, x: number_default(ya, yb) });
    } else if (xb !== 1 || yb !== 1) {
      s.push(pop(s) + "scale(" + xb + "," + yb + ")");
    }
  }
  return function(a, b) {
    var s = [], q = [];
    a = parse(a), b = parse(b);
    translate(a.translateX, a.translateY, b.translateX, b.translateY, s, q);
    rotate(a.rotate, b.rotate, s, q);
    skewX(a.skewX, b.skewX, s, q);
    scale(a.scaleX, a.scaleY, b.scaleX, b.scaleY, s, q);
    a = b = null;
    return function(t) {
      var i = -1, n = q.length, o;
      while (++i < n)
        s[(o = q[i]).i] = o.x(t);
      return s.join("");
    };
  };
}
var interpolateTransformCss = interpolateTransform(parseCss, "px, ", "px)", "deg)");
var interpolateTransformSvg = interpolateTransform(parseSvg, ", ", ")", ")");
// node_modules/d3-interpolate/src/zoom.js
var epsilon2 = 0.000000000001;
function cosh(x) {
  return ((x = Math.exp(x)) + 1 / x) / 2;
}
function sinh(x) {
  return ((x = Math.exp(x)) - 1 / x) / 2;
}
function tanh(x) {
  return ((x = Math.exp(2 * x)) - 1) / (x + 1);
}
var zoom_default = function zoomRho(rho, rho2, rho4) {
  function zoom(p0, p1) {
    var ux0 = p0[0], uy0 = p0[1], w0 = p0[2], ux1 = p1[0], uy1 = p1[1], w1 = p1[2], dx = ux1 - ux0, dy = uy1 - uy0, d2 = dx * dx + dy * dy, i, S;
    if (d2 < epsilon2) {
      S = Math.log(w1 / w0) / rho;
      i = function(t) {
        return [
          ux0 + t * dx,
          uy0 + t * dy,
          w0 * Math.exp(rho * t * S)
        ];
      };
    } else {
      var d1 = Math.sqrt(d2), b0 = (w1 * w1 - w0 * w0 + rho4 * d2) / (2 * w0 * rho2 * d1), b1 = (w1 * w1 - w0 * w0 - rho4 * d2) / (2 * w1 * rho2 * d1), r0 = Math.log(Math.sqrt(b0 * b0 + 1) - b0), r1 = Math.log(Math.sqrt(b1 * b1 + 1) - b1);
      S = (r1 - r0) / rho;
      i = function(t) {
        var s = t * S, coshr0 = cosh(r0), u = w0 / (rho2 * d1) * (coshr0 * tanh(rho * s + r0) - sinh(r0));
        return [
          ux0 + u * dx,
          uy0 + u * dy,
          w0 * coshr0 / cosh(rho * s + r0)
        ];
      };
    }
    i.duration = S * 1000 * rho / Math.SQRT2;
    return i;
  }
  zoom.rho = function(_) {
    var _1 = Math.max(0.001, +_), _2 = _1 * _1, _4 = _2 * _2;
    return zoomRho(_1, _2, _4);
  };
  return zoom;
}(Math.SQRT2, 2, 4);
// node_modules/d3-interpolate/src/hsl.js
function hsl2(hue2) {
  return function(start, end) {
    var h = hue2((start = hsl(start)).h, (end = hsl(end)).h), s = nogamma(start.s, end.s), l = nogamma(start.l, end.l), opacity = nogamma(start.opacity, end.opacity);
    return function(t) {
      start.h = h(t);
      start.s = s(t);
      start.l = l(t);
      start.opacity = opacity(t);
      return start + "";
    };
  };
}
var hsl_default = hsl2(hue);
var hslLong = hsl2(nogamma);
// node_modules/d3-interpolate/src/lab.js
function lab2(start, end) {
  var l = nogamma((start = lab(start)).l, (end = lab(end)).l), a = nogamma(start.a, end.a), b = nogamma(start.b, end.b), opacity = nogamma(start.opacity, end.opacity);
  return function(t) {
    start.l = l(t);
    start.a = a(t);
    start.b = b(t);
    start.opacity = opacity(t);
    return start + "";
  };
}
// node_modules/d3-interpolate/src/hcl.js
function hcl2(hue2) {
  return function(start, end) {
    var h = hue2((start = hcl(start)).h, (end = hcl(end)).h), c = nogamma(start.c, end.c), l = nogamma(start.l, end.l), opacity = nogamma(start.opacity, end.opacity);
    return function(t) {
      start.h = h(t);
      start.c = c(t);
      start.l = l(t);
      start.opacity = opacity(t);
      return start + "";
    };
  };
}
var hcl_default = hcl2(hue);
var hclLong = hcl2(nogamma);
// node_modules/d3-interpolate/src/cubehelix.js
function cubehelix2(hue2) {
  return function cubehelixGamma(y) {
    y = +y;
    function cubehelix3(start, end) {
      var h = hue2((start = cubehelix(start)).h, (end = cubehelix(end)).h), s = nogamma(start.s, end.s), l = nogamma(start.l, end.l), opacity = nogamma(start.opacity, end.opacity);
      return function(t) {
        start.h = h(t);
        start.s = s(t);
        start.l = l(Math.pow(t, y));
        start.opacity = opacity(t);
        return start + "";
      };
    }
    cubehelix3.gamma = cubehelixGamma;
    return cubehelix3;
  }(1);
}
var cubehelix_default = cubehelix2(hue);
var cubehelixLong = cubehelix2(nogamma);
// node_modules/d3-interpolate/src/piecewise.js
function piecewise(interpolate, values) {
  if (values === undefined)
    values = interpolate, interpolate = value_default;
  var i = 0, n = values.length - 1, v = values[0], I = new Array(n < 0 ? 0 : n);
  while (i < n)
    I[i] = interpolate(v, v = values[++i]);
  return function(t) {
    var i2 = Math.max(0, Math.min(n - 1, Math.floor(t *= n)));
    return I[i2](t - i2);
  };
}
// node_modules/d3-interpolate/src/quantize.js
function quantize_default(interpolator, n) {
  var samples = new Array(n);
  for (var i = 0;i < n; ++i)
    samples[i] = interpolator(i / (n - 1));
  return samples;
}
// node_modules/d3-timer/src/timer.js
var frame = 0;
var timeout = 0;
var interval = 0;
var pokeDelay = 1000;
var taskHead;
var taskTail;
var clockLast = 0;
var clockNow = 0;
var clockSkew = 0;
var clock = typeof performance === "object" && performance.now ? performance : Date;
var setFrame = typeof window === "object" && window.requestAnimationFrame ? window.requestAnimationFrame.bind(window) : function(f) {
  setTimeout(f, 17);
};
function now() {
  return clockNow || (setFrame(clearNow), clockNow = clock.now() + clockSkew);
}
function clearNow() {
  clockNow = 0;
}
function Timer() {
  this._call = this._time = this._next = null;
}
Timer.prototype = timer.prototype = {
  constructor: Timer,
  restart: function(callback, delay, time) {
    if (typeof callback !== "function")
      throw new TypeError("callback is not a function");
    time = (time == null ? now() : +time) + (delay == null ? 0 : +delay);
    if (!this._next && taskTail !== this) {
      if (taskTail)
        taskTail._next = this;
      else
        taskHead = this;
      taskTail = this;
    }
    this._call = callback;
    this._time = time;
    sleep();
  },
  stop: function() {
    if (this._call) {
      this._call = null;
      this._time = Infinity;
      sleep();
    }
  }
};
function timer(callback, delay, time) {
  var t = new Timer;
  t.restart(callback, delay, time);
  return t;
}
function timerFlush() {
  now();
  ++frame;
  var t = taskHead, e;
  while (t) {
    if ((e = clockNow - t._time) >= 0)
      t._call.call(undefined, e);
    t = t._next;
  }
  --frame;
}
function wake() {
  clockNow = (clockLast = clock.now()) + clockSkew;
  frame = timeout = 0;
  try {
    timerFlush();
  } finally {
    frame = 0;
    nap();
    clockNow = 0;
  }
}
function poke() {
  var now2 = clock.now(), delay = now2 - clockLast;
  if (delay > pokeDelay)
    clockSkew -= delay, clockLast = now2;
}
function nap() {
  var t02, t12 = taskHead, t22, time = Infinity;
  while (t12) {
    if (t12._call) {
      if (time > t12._time)
        time = t12._time;
      t02 = t12, t12 = t12._next;
    } else {
      t22 = t12._next, t12._next = null;
      t12 = t02 ? t02._next = t22 : taskHead = t22;
    }
  }
  taskTail = t02;
  sleep(time);
}
function sleep(time) {
  if (frame)
    return;
  if (timeout)
    timeout = clearTimeout(timeout);
  var delay = time - clockNow;
  if (delay > 24) {
    if (time < Infinity)
      timeout = setTimeout(wake, time - clock.now() - clockSkew);
    if (interval)
      interval = clearInterval(interval);
  } else {
    if (!interval)
      clockLast = clock.now(), interval = setInterval(poke, pokeDelay);
    frame = 1, setFrame(wake);
  }
}
// node_modules/d3-timer/src/timeout.js
function timeout_default(callback, delay, time) {
  var t = new Timer;
  delay = delay == null ? 0 : +delay;
  t.restart((elapsed) => {
    t.stop();
    callback(elapsed + delay);
  }, delay, time);
  return t;
}
// node_modules/d3-timer/src/interval.js
function interval_default(callback, delay, time) {
  var t = new Timer, total = delay;
  if (delay == null)
    return t.restart(callback, delay, time), t;
  t._restart = t.restart;
  t.restart = function(callback2, delay2, time2) {
    delay2 = +delay2, time2 = time2 == null ? now() : +time2;
    t._restart(function tick(elapsed) {
      elapsed += total;
      t._restart(tick, total += delay2, time2);
      callback2(elapsed);
    }, delay2, time2);
  };
  t.restart(callback, delay, time);
  return t;
}
// node_modules/d3-transition/src/transition/schedule.js
var emptyOn = dispatch_default("start", "end", "cancel", "interrupt");
var emptyTween = [];
var CREATED = 0;
var SCHEDULED = 1;
var STARTING = 2;
var STARTED = 3;
var RUNNING = 4;
var ENDING = 5;
var ENDED = 6;
function schedule_default(node, name, id, index2, group2, timing) {
  var schedules = node.__transition;
  if (!schedules)
    node.__transition = {};
  else if (id in schedules)
    return;
  create(node, id, {
    name,
    index: index2,
    group: group2,
    on: emptyOn,
    tween: emptyTween,
    time: timing.time,
    delay: timing.delay,
    duration: timing.duration,
    ease: timing.ease,
    timer: null,
    state: CREATED
  });
}
function init(node, id) {
  var schedule = get2(node, id);
  if (schedule.state > CREATED)
    throw new Error("too late; already scheduled");
  return schedule;
}
function set3(node, id) {
  var schedule = get2(node, id);
  if (schedule.state > STARTED)
    throw new Error("too late; already running");
  return schedule;
}
function get2(node, id) {
  var schedule = node.__transition;
  if (!schedule || !(schedule = schedule[id]))
    throw new Error("transition not found");
  return schedule;
}
function create(node, id, self) {
  var schedules = node.__transition, tween;
  schedules[id] = self;
  self.timer = timer(schedule, 0, self.time);
  function schedule(elapsed) {
    self.state = SCHEDULED;
    self.timer.restart(start, self.delay, self.time);
    if (self.delay <= elapsed)
      start(elapsed - self.delay);
  }
  function start(elapsed) {
    var i, j, n, o;
    if (self.state !== SCHEDULED)
      return stop();
    for (i in schedules) {
      o = schedules[i];
      if (o.name !== self.name)
        continue;
      if (o.state === STARTED)
        return timeout_default(start);
      if (o.state === RUNNING) {
        o.state = ENDED;
        o.timer.stop();
        o.on.call("interrupt", node, node.__data__, o.index, o.group);
        delete schedules[i];
      } else if (+i < id) {
        o.state = ENDED;
        o.timer.stop();
        o.on.call("cancel", node, node.__data__, o.index, o.group);
        delete schedules[i];
      }
    }
    timeout_default(function() {
      if (self.state === STARTED) {
        self.state = RUNNING;
        self.timer.restart(tick, self.delay, self.time);
        tick(elapsed);
      }
    });
    self.state = STARTING;
    self.on.call("start", node, node.__data__, self.index, self.group);
    if (self.state !== STARTING)
      return;
    self.state = STARTED;
    tween = new Array(n = self.tween.length);
    for (i = 0, j = -1;i < n; ++i) {
      if (o = self.tween[i].value.call(node, node.__data__, self.index, self.group)) {
        tween[++j] = o;
      }
    }
    tween.length = j + 1;
  }
  function tick(elapsed) {
    var t = elapsed < self.duration ? self.ease.call(null, elapsed / self.duration) : (self.timer.restart(stop), self.state = ENDING, 1), i = -1, n = tween.length;
    while (++i < n) {
      tween[i].call(node, t);
    }
    if (self.state === ENDING) {
      self.on.call("end", node, node.__data__, self.index, self.group);
      stop();
    }
  }
  function stop() {
    self.state = ENDED;
    self.timer.stop();
    delete schedules[id];
    for (var i in schedules)
      return;
    delete node.__transition;
  }
}

// node_modules/d3-transition/src/interrupt.js
function interrupt_default(node, name) {
  var schedules = node.__transition, schedule, active, empty3 = true, i;
  if (!schedules)
    return;
  name = name == null ? null : name + "";
  for (i in schedules) {
    if ((schedule = schedules[i]).name !== name) {
      empty3 = false;
      continue;
    }
    active = schedule.state > STARTING && schedule.state < ENDING;
    schedule.state = ENDED;
    schedule.timer.stop();
    schedule.on.call(active ? "interrupt" : "cancel", node, node.__data__, schedule.index, schedule.group);
    delete schedules[i];
  }
  if (empty3)
    delete node.__transition;
}

// node_modules/d3-transition/src/selection/interrupt.js
function interrupt_default2(name) {
  return this.each(function() {
    interrupt_default(this, name);
  });
}

// node_modules/d3-transition/src/transition/tween.js
function tweenRemove(id, name) {
  var tween0, tween1;
  return function() {
    var schedule = set3(this, id), tween = schedule.tween;
    if (tween !== tween0) {
      tween1 = tween0 = tween;
      for (var i = 0, n = tween1.length;i < n; ++i) {
        if (tween1[i].name === name) {
          tween1 = tween1.slice();
          tween1.splice(i, 1);
          break;
        }
      }
    }
    schedule.tween = tween1;
  };
}
function tweenFunction(id, name, value) {
  var tween0, tween1;
  if (typeof value !== "function")
    throw new Error;
  return function() {
    var schedule = set3(this, id), tween = schedule.tween;
    if (tween !== tween0) {
      tween1 = (tween0 = tween).slice();
      for (var t = { name, value }, i = 0, n = tween1.length;i < n; ++i) {
        if (tween1[i].name === name) {
          tween1[i] = t;
          break;
        }
      }
      if (i === n)
        tween1.push(t);
    }
    schedule.tween = tween1;
  };
}
function tween_default(name, value) {
  var id = this._id;
  name += "";
  if (arguments.length < 2) {
    var tween = get2(this.node(), id).tween;
    for (var i = 0, n = tween.length, t;i < n; ++i) {
      if ((t = tween[i]).name === name) {
        return t.value;
      }
    }
    return null;
  }
  return this.each((value == null ? tweenRemove : tweenFunction)(id, name, value));
}
function tweenValue(transition, name, value) {
  var id = transition._id;
  transition.each(function() {
    var schedule = set3(this, id);
    (schedule.value || (schedule.value = {}))[name] = value.apply(this, arguments);
  });
  return function(node) {
    return get2(node, id).value[name];
  };
}

// node_modules/d3-transition/src/transition/interpolate.js
function interpolate_default(a, b) {
  var c;
  return (typeof b === "number" ? number_default : b instanceof color ? rgb_default : (c = color(b)) ? (b = c, rgb_default) : string_default)(a, b);
}

// node_modules/d3-transition/src/transition/attr.js
function attrRemove2(name) {
  return function() {
    this.removeAttribute(name);
  };
}
function attrRemoveNS2(fullname) {
  return function() {
    this.removeAttributeNS(fullname.space, fullname.local);
  };
}
function attrConstant2(name, interpolate, value1) {
  var string00, string1 = value1 + "", interpolate0;
  return function() {
    var string0 = this.getAttribute(name);
    return string0 === string1 ? null : string0 === string00 ? interpolate0 : interpolate0 = interpolate(string00 = string0, value1);
  };
}
function attrConstantNS2(fullname, interpolate, value1) {
  var string00, string1 = value1 + "", interpolate0;
  return function() {
    var string0 = this.getAttributeNS(fullname.space, fullname.local);
    return string0 === string1 ? null : string0 === string00 ? interpolate0 : interpolate0 = interpolate(string00 = string0, value1);
  };
}
function attrFunction2(name, interpolate, value) {
  var string00, string10, interpolate0;
  return function() {
    var string0, value1 = value(this), string1;
    if (value1 == null)
      return void this.removeAttribute(name);
    string0 = this.getAttribute(name);
    string1 = value1 + "";
    return string0 === string1 ? null : string0 === string00 && string1 === string10 ? interpolate0 : (string10 = string1, interpolate0 = interpolate(string00 = string0, value1));
  };
}
function attrFunctionNS2(fullname, interpolate, value) {
  var string00, string10, interpolate0;
  return function() {
    var string0, value1 = value(this), string1;
    if (value1 == null)
      return void this.removeAttributeNS(fullname.space, fullname.local);
    string0 = this.getAttributeNS(fullname.space, fullname.local);
    string1 = value1 + "";
    return string0 === string1 ? null : string0 === string00 && string1 === string10 ? interpolate0 : (string10 = string1, interpolate0 = interpolate(string00 = string0, value1));
  };
}
function attr_default2(name, value) {
  var fullname = namespace_default(name), i = fullname === "transform" ? interpolateTransformSvg : interpolate_default;
  return this.attrTween(name, typeof value === "function" ? (fullname.local ? attrFunctionNS2 : attrFunction2)(fullname, i, tweenValue(this, "attr." + name, value)) : value == null ? (fullname.local ? attrRemoveNS2 : attrRemove2)(fullname) : (fullname.local ? attrConstantNS2 : attrConstant2)(fullname, i, value));
}

// node_modules/d3-transition/src/transition/attrTween.js
function attrInterpolate(name, i) {
  return function(t) {
    this.setAttribute(name, i.call(this, t));
  };
}
function attrInterpolateNS(fullname, i) {
  return function(t) {
    this.setAttributeNS(fullname.space, fullname.local, i.call(this, t));
  };
}
function attrTweenNS(fullname, value) {
  var t02, i0;
  function tween() {
    var i = value.apply(this, arguments);
    if (i !== i0)
      t02 = (i0 = i) && attrInterpolateNS(fullname, i);
    return t02;
  }
  tween._value = value;
  return tween;
}
function attrTween(name, value) {
  var t02, i0;
  function tween() {
    var i = value.apply(this, arguments);
    if (i !== i0)
      t02 = (i0 = i) && attrInterpolate(name, i);
    return t02;
  }
  tween._value = value;
  return tween;
}
function attrTween_default(name, value) {
  var key = "attr." + name;
  if (arguments.length < 2)
    return (key = this.tween(key)) && key._value;
  if (value == null)
    return this.tween(key, null);
  if (typeof value !== "function")
    throw new Error;
  var fullname = namespace_default(name);
  return this.tween(key, (fullname.local ? attrTweenNS : attrTween)(fullname, value));
}

// node_modules/d3-transition/src/transition/delay.js
function delayFunction(id, value) {
  return function() {
    init(this, id).delay = +value.apply(this, arguments);
  };
}
function delayConstant(id, value) {
  return value = +value, function() {
    init(this, id).delay = value;
  };
}
function delay_default(value) {
  var id = this._id;
  return arguments.length ? this.each((typeof value === "function" ? delayFunction : delayConstant)(id, value)) : get2(this.node(), id).delay;
}

// node_modules/d3-transition/src/transition/duration.js
function durationFunction(id, value) {
  return function() {
    set3(this, id).duration = +value.apply(this, arguments);
  };
}
function durationConstant(id, value) {
  return value = +value, function() {
    set3(this, id).duration = value;
  };
}
function duration_default(value) {
  var id = this._id;
  return arguments.length ? this.each((typeof value === "function" ? durationFunction : durationConstant)(id, value)) : get2(this.node(), id).duration;
}

// node_modules/d3-transition/src/transition/ease.js
function easeConstant(id, value) {
  if (typeof value !== "function")
    throw new Error;
  return function() {
    set3(this, id).ease = value;
  };
}
function ease_default(value) {
  var id = this._id;
  return arguments.length ? this.each(easeConstant(id, value)) : get2(this.node(), id).ease;
}

// node_modules/d3-transition/src/transition/easeVarying.js
function easeVarying(id, value) {
  return function() {
    var v = value.apply(this, arguments);
    if (typeof v !== "function")
      throw new Error;
    set3(this, id).ease = v;
  };
}
function easeVarying_default(value) {
  if (typeof value !== "function")
    throw new Error;
  return this.each(easeVarying(this._id, value));
}

// node_modules/d3-transition/src/transition/filter.js
function filter_default2(match) {
  if (typeof match !== "function")
    match = matcher_default(match);
  for (var groups2 = this._groups, m = groups2.length, subgroups = new Array(m), j = 0;j < m; ++j) {
    for (var group2 = groups2[j], n = group2.length, subgroup = subgroups[j] = [], node, i = 0;i < n; ++i) {
      if ((node = group2[i]) && match.call(node, node.__data__, i, group2)) {
        subgroup.push(node);
      }
    }
  }
  return new Transition(subgroups, this._parents, this._name, this._id);
}

// node_modules/d3-transition/src/transition/merge.js
function merge_default2(transition) {
  if (transition._id !== this._id)
    throw new Error;
  for (var groups0 = this._groups, groups1 = transition._groups, m0 = groups0.length, m1 = groups1.length, m = Math.min(m0, m1), merges = new Array(m0), j = 0;j < m; ++j) {
    for (var group0 = groups0[j], group1 = groups1[j], n = group0.length, merge2 = merges[j] = new Array(n), node, i = 0;i < n; ++i) {
      if (node = group0[i] || group1[i]) {
        merge2[i] = node;
      }
    }
  }
  for (;j < m0; ++j) {
    merges[j] = groups0[j];
  }
  return new Transition(merges, this._parents, this._name, this._id);
}

// node_modules/d3-transition/src/transition/on.js
function start(name) {
  return (name + "").trim().split(/^|\s+/).every(function(t) {
    var i = t.indexOf(".");
    if (i >= 0)
      t = t.slice(0, i);
    return !t || t === "start";
  });
}
function onFunction(id, name, listener) {
  var on0, on1, sit = start(name) ? init : set3;
  return function() {
    var schedule = sit(this, id), on = schedule.on;
    if (on !== on0)
      (on1 = (on0 = on).copy()).on(name, listener);
    schedule.on = on1;
  };
}
function on_default2(name, listener) {
  var id = this._id;
  return arguments.length < 2 ? get2(this.node(), id).on.on(name) : this.each(onFunction(id, name, listener));
}

// node_modules/d3-transition/src/transition/remove.js
function removeFunction(id) {
  return function() {
    var parent = this.parentNode;
    for (var i in this.__transition)
      if (+i !== id)
        return;
    if (parent)
      parent.removeChild(this);
  };
}
function remove_default2() {
  return this.on("end.remove", removeFunction(this._id));
}

// node_modules/d3-transition/src/transition/select.js
function select_default3(select) {
  var name = this._name, id = this._id;
  if (typeof select !== "function")
    select = selector_default(select);
  for (var groups2 = this._groups, m = groups2.length, subgroups = new Array(m), j = 0;j < m; ++j) {
    for (var group2 = groups2[j], n = group2.length, subgroup = subgroups[j] = new Array(n), node, subnode, i = 0;i < n; ++i) {
      if ((node = group2[i]) && (subnode = select.call(node, node.__data__, i, group2))) {
        if ("__data__" in node)
          subnode.__data__ = node.__data__;
        subgroup[i] = subnode;
        schedule_default(subgroup[i], name, id, i, subgroup, get2(node, id));
      }
    }
  }
  return new Transition(subgroups, this._parents, name, id);
}

// node_modules/d3-transition/src/transition/selectAll.js
function selectAll_default3(select) {
  var name = this._name, id = this._id;
  if (typeof select !== "function")
    select = selectorAll_default(select);
  for (var groups2 = this._groups, m = groups2.length, subgroups = [], parents = [], j = 0;j < m; ++j) {
    for (var group2 = groups2[j], n = group2.length, node, i = 0;i < n; ++i) {
      if (node = group2[i]) {
        for (var children2 = select.call(node, node.__data__, i, group2), child, inherit = get2(node, id), k = 0, l = children2.length;k < l; ++k) {
          if (child = children2[k]) {
            schedule_default(child, name, id, k, children2, inherit);
          }
        }
        subgroups.push(children2);
        parents.push(node);
      }
    }
  }
  return new Transition(subgroups, parents, name, id);
}

// node_modules/d3-transition/src/transition/selection.js
var Selection2 = selection_default.prototype.constructor;
function selection_default2() {
  return new Selection2(this._groups, this._parents);
}

// node_modules/d3-transition/src/transition/style.js
function styleNull(name, interpolate) {
  var string00, string10, interpolate0;
  return function() {
    var string0 = styleValue(this, name), string1 = (this.style.removeProperty(name), styleValue(this, name));
    return string0 === string1 ? null : string0 === string00 && string1 === string10 ? interpolate0 : interpolate0 = interpolate(string00 = string0, string10 = string1);
  };
}
function styleRemove2(name) {
  return function() {
    this.style.removeProperty(name);
  };
}
function styleConstant2(name, interpolate, value1) {
  var string00, string1 = value1 + "", interpolate0;
  return function() {
    var string0 = styleValue(this, name);
    return string0 === string1 ? null : string0 === string00 ? interpolate0 : interpolate0 = interpolate(string00 = string0, value1);
  };
}
function styleFunction2(name, interpolate, value) {
  var string00, string10, interpolate0;
  return function() {
    var string0 = styleValue(this, name), value1 = value(this), string1 = value1 + "";
    if (value1 == null)
      string1 = value1 = (this.style.removeProperty(name), styleValue(this, name));
    return string0 === string1 ? null : string0 === string00 && string1 === string10 ? interpolate0 : (string10 = string1, interpolate0 = interpolate(string00 = string0, value1));
  };
}
function styleMaybeRemove(id, name) {
  var on0, on1, listener0, key = "style." + name, event = "end." + key, remove2;
  return function() {
    var schedule = set3(this, id), on = schedule.on, listener = schedule.value[key] == null ? remove2 || (remove2 = styleRemove2(name)) : undefined;
    if (on !== on0 || listener0 !== listener)
      (on1 = (on0 = on).copy()).on(event, listener0 = listener);
    schedule.on = on1;
  };
}
function style_default2(name, value, priority) {
  var i = (name += "") === "transform" ? interpolateTransformCss : interpolate_default;
  return value == null ? this.styleTween(name, styleNull(name, i)).on("end.style." + name, styleRemove2(name)) : typeof value === "function" ? this.styleTween(name, styleFunction2(name, i, tweenValue(this, "style." + name, value))).each(styleMaybeRemove(this._id, name)) : this.styleTween(name, styleConstant2(name, i, value), priority).on("end.style." + name, null);
}

// node_modules/d3-transition/src/transition/styleTween.js
function styleInterpolate(name, i, priority) {
  return function(t) {
    this.style.setProperty(name, i.call(this, t), priority);
  };
}
function styleTween(name, value, priority) {
  var t, i0;
  function tween() {
    var i = value.apply(this, arguments);
    if (i !== i0)
      t = (i0 = i) && styleInterpolate(name, i, priority);
    return t;
  }
  tween._value = value;
  return tween;
}
function styleTween_default(name, value, priority) {
  var key = "style." + (name += "");
  if (arguments.length < 2)
    return (key = this.tween(key)) && key._value;
  if (value == null)
    return this.tween(key, null);
  if (typeof value !== "function")
    throw new Error;
  return this.tween(key, styleTween(name, value, priority == null ? "" : priority));
}

// node_modules/d3-transition/src/transition/text.js
function textConstant2(value) {
  return function() {
    this.textContent = value;
  };
}
function textFunction2(value) {
  return function() {
    var value1 = value(this);
    this.textContent = value1 == null ? "" : value1;
  };
}
function text_default2(value) {
  return this.tween("text", typeof value === "function" ? textFunction2(tweenValue(this, "text", value)) : textConstant2(value == null ? "" : value + ""));
}

// node_modules/d3-transition/src/transition/textTween.js
function textInterpolate(i) {
  return function(t) {
    this.textContent = i.call(this, t);
  };
}
function textTween(value) {
  var t02, i0;
  function tween() {
    var i = value.apply(this, arguments);
    if (i !== i0)
      t02 = (i0 = i) && textInterpolate(i);
    return t02;
  }
  tween._value = value;
  return tween;
}
function textTween_default(value) {
  var key = "text";
  if (arguments.length < 1)
    return (key = this.tween(key)) && key._value;
  if (value == null)
    return this.tween(key, null);
  if (typeof value !== "function")
    throw new Error;
  return this.tween(key, textTween(value));
}

// node_modules/d3-transition/src/transition/transition.js
function transition_default() {
  var name = this._name, id0 = this._id, id1 = newId();
  for (var groups2 = this._groups, m = groups2.length, j = 0;j < m; ++j) {
    for (var group2 = groups2[j], n = group2.length, node, i = 0;i < n; ++i) {
      if (node = group2[i]) {
        var inherit = get2(node, id0);
        schedule_default(node, name, id1, i, group2, {
          time: inherit.time + inherit.delay + inherit.duration,
          delay: 0,
          duration: inherit.duration,
          ease: inherit.ease
        });
      }
    }
  }
  return new Transition(groups2, this._parents, name, id1);
}

// node_modules/d3-transition/src/transition/end.js
function end_default() {
  var on0, on1, that = this, id = that._id, size = that.size();
  return new Promise(function(resolve, reject) {
    var cancel = { value: reject }, end = { value: function() {
      if (--size === 0)
        resolve();
    } };
    that.each(function() {
      var schedule = set3(this, id), on = schedule.on;
      if (on !== on0) {
        on1 = (on0 = on).copy();
        on1._.cancel.push(cancel);
        on1._.interrupt.push(cancel);
        on1._.end.push(end);
      }
      schedule.on = on1;
    });
    if (size === 0)
      resolve();
  });
}

// node_modules/d3-transition/src/transition/index.js
var id = 0;
function Transition(groups2, parents, name, id2) {
  this._groups = groups2;
  this._parents = parents;
  this._name = name;
  this._id = id2;
}
function transition(name) {
  return selection_default().transition(name);
}
function newId() {
  return ++id;
}
var selection_prototype = selection_default.prototype;
Transition.prototype = transition.prototype = {
  constructor: Transition,
  select: select_default3,
  selectAll: selectAll_default3,
  selectChild: selection_prototype.selectChild,
  selectChildren: selection_prototype.selectChildren,
  filter: filter_default2,
  merge: merge_default2,
  selection: selection_default2,
  transition: transition_default,
  call: selection_prototype.call,
  nodes: selection_prototype.nodes,
  node: selection_prototype.node,
  size: selection_prototype.size,
  empty: selection_prototype.empty,
  each: selection_prototype.each,
  on: on_default2,
  attr: attr_default2,
  attrTween: attrTween_default,
  style: style_default2,
  styleTween: styleTween_default,
  text: text_default2,
  textTween: textTween_default,
  remove: remove_default2,
  tween: tween_default,
  delay: delay_default,
  duration: duration_default,
  ease: ease_default,
  easeVarying: easeVarying_default,
  end: end_default,
  [Symbol.iterator]: selection_prototype[Symbol.iterator]
};

// node_modules/d3-ease/src/linear.js
var linear2 = (t) => +t;
// node_modules/d3-ease/src/quad.js
function quadIn(t) {
  return t * t;
}
function quadOut(t) {
  return t * (2 - t);
}
function quadInOut(t) {
  return ((t *= 2) <= 1 ? t * t : --t * (2 - t) + 1) / 2;
}
// node_modules/d3-ease/src/cubic.js
function cubicIn(t) {
  return t * t * t;
}
function cubicOut(t) {
  return --t * t * t + 1;
}
function cubicInOut(t) {
  return ((t *= 2) <= 1 ? t * t * t : (t -= 2) * t * t + 2) / 2;
}
// node_modules/d3-ease/src/poly.js
var exponent = 3;
var polyIn = function custom(e) {
  e = +e;
  function polyIn2(t) {
    return Math.pow(t, e);
  }
  polyIn2.exponent = custom;
  return polyIn2;
}(exponent);
var polyOut = function custom2(e) {
  e = +e;
  function polyOut2(t) {
    return 1 - Math.pow(1 - t, e);
  }
  polyOut2.exponent = custom2;
  return polyOut2;
}(exponent);
var polyInOut = function custom3(e) {
  e = +e;
  function polyInOut2(t) {
    return ((t *= 2) <= 1 ? Math.pow(t, e) : 2 - Math.pow(2 - t, e)) / 2;
  }
  polyInOut2.exponent = custom3;
  return polyInOut2;
}(exponent);
// node_modules/d3-ease/src/sin.js
var pi = Math.PI;
var halfPi = pi / 2;
function sinIn(t) {
  return +t === 1 ? 1 : 1 - Math.cos(t * halfPi);
}
function sinOut(t) {
  return Math.sin(t * halfPi);
}
function sinInOut(t) {
  return (1 - Math.cos(pi * t)) / 2;
}
// node_modules/d3-ease/src/math.js
function tpmt(x) {
  return (Math.pow(2, -10 * x) - 0.0009765625) * 1.0009775171065494;
}

// node_modules/d3-ease/src/exp.js
function expIn(t) {
  return tpmt(1 - +t);
}
function expOut(t) {
  return 1 - tpmt(t);
}
function expInOut(t) {
  return ((t *= 2) <= 1 ? tpmt(1 - t) : 2 - tpmt(t - 1)) / 2;
}
// node_modules/d3-ease/src/circle.js
function circleIn(t) {
  return 1 - Math.sqrt(1 - t * t);
}
function circleOut(t) {
  return Math.sqrt(1 - --t * t);
}
function circleInOut(t) {
  return ((t *= 2) <= 1 ? 1 - Math.sqrt(1 - t * t) : Math.sqrt(1 - (t -= 2) * t) + 1) / 2;
}
// node_modules/d3-ease/src/bounce.js
var b1 = 4 / 11;
var b2 = 6 / 11;
var b3 = 8 / 11;
var b4 = 3 / 4;
var b5 = 9 / 11;
var b6 = 10 / 11;
var b7 = 15 / 16;
var b8 = 21 / 22;
var b9 = 63 / 64;
var b0 = 1 / b1 / b1;
function bounceIn(t) {
  return 1 - bounceOut(1 - t);
}
function bounceOut(t) {
  return (t = +t) < b1 ? b0 * t * t : t < b3 ? b0 * (t -= b2) * t + b4 : t < b6 ? b0 * (t -= b5) * t + b7 : b0 * (t -= b8) * t + b9;
}
function bounceInOut(t) {
  return ((t *= 2) <= 1 ? 1 - bounceOut(1 - t) : bounceOut(t - 1) + 1) / 2;
}
// node_modules/d3-ease/src/back.js
var overshoot = 1.70158;
var backIn = function custom4(s) {
  s = +s;
  function backIn2(t) {
    return (t = +t) * t * (s * (t - 1) + t);
  }
  backIn2.overshoot = custom4;
  return backIn2;
}(overshoot);
var backOut = function custom5(s) {
  s = +s;
  function backOut2(t) {
    return --t * t * ((t + 1) * s + t) + 1;
  }
  backOut2.overshoot = custom5;
  return backOut2;
}(overshoot);
var backInOut = function custom6(s) {
  s = +s;
  function backInOut2(t) {
    return ((t *= 2) < 1 ? t * t * ((s + 1) * t - s) : (t -= 2) * t * ((s + 1) * t + s) + 2) / 2;
  }
  backInOut2.overshoot = custom6;
  return backInOut2;
}(overshoot);
// node_modules/d3-ease/src/elastic.js
var tau = 2 * Math.PI;
var amplitude = 1;
var period = 0.3;
var elasticIn = function custom7(a, p) {
  var s = Math.asin(1 / (a = Math.max(1, a))) * (p /= tau);
  function elasticIn2(t) {
    return a * tpmt(- --t) * Math.sin((s - t) / p);
  }
  elasticIn2.amplitude = function(a2) {
    return custom7(a2, p * tau);
  };
  elasticIn2.period = function(p2) {
    return custom7(a, p2);
  };
  return elasticIn2;
}(amplitude, period);
var elasticOut = function custom8(a, p) {
  var s = Math.asin(1 / (a = Math.max(1, a))) * (p /= tau);
  function elasticOut2(t) {
    return 1 - a * tpmt(t = +t) * Math.sin((t + s) / p);
  }
  elasticOut2.amplitude = function(a2) {
    return custom8(a2, p * tau);
  };
  elasticOut2.period = function(p2) {
    return custom8(a, p2);
  };
  return elasticOut2;
}(amplitude, period);
var elasticInOut = function custom9(a, p) {
  var s = Math.asin(1 / (a = Math.max(1, a))) * (p /= tau);
  function elasticInOut2(t) {
    return ((t = t * 2 - 1) < 0 ? a * tpmt(-t) * Math.sin((s - t) / p) : 2 - a * tpmt(t) * Math.sin((s + t) / p)) / 2;
  }
  elasticInOut2.amplitude = function(a2) {
    return custom9(a2, p * tau);
  };
  elasticInOut2.period = function(p2) {
    return custom9(a, p2);
  };
  return elasticInOut2;
}(amplitude, period);
// node_modules/d3-transition/src/selection/transition.js
var defaultTiming = {
  time: null,
  delay: 0,
  duration: 250,
  ease: cubicInOut
};
function inherit(node, id2) {
  var timing;
  while (!(timing = node.__transition) || !(timing = timing[id2])) {
    if (!(node = node.parentNode)) {
      throw new Error(`transition ${id2} not found`);
    }
  }
  return timing;
}
function transition_default2(name) {
  var id2, timing;
  if (name instanceof Transition) {
    id2 = name._id, name = name._name;
  } else {
    id2 = newId(), (timing = defaultTiming).time = now(), name = name == null ? null : name + "";
  }
  for (var groups2 = this._groups, m = groups2.length, j = 0;j < m; ++j) {
    for (var group2 = groups2[j], n = group2.length, node, i = 0;i < n; ++i) {
      if (node = group2[i]) {
        schedule_default(node, name, id2, i, group2, timing || inherit(node, id2));
      }
    }
  }
  return new Transition(groups2, this._parents, name, id2);
}

// node_modules/d3-transition/src/selection/index.js
selection_default.prototype.interrupt = interrupt_default2;
selection_default.prototype.transition = transition_default2;
// node_modules/d3-transition/src/active.js
var root2 = [null];
function active_default(node, name) {
  var schedules = node.__transition, schedule, i;
  if (schedules) {
    name = name == null ? null : name + "";
    for (i in schedules) {
      if ((schedule = schedules[i]).state > SCHEDULED && schedule.name === name) {
        return new Transition([[node]], root2, name, +i);
      }
    }
  }
  return null;
}
// node_modules/d3-brush/src/constant.js
var constant_default4 = (x) => () => x;

// node_modules/d3-brush/src/event.js
function BrushEvent(type, {
  sourceEvent,
  target,
  selection: selection2,
  mode: mode2,
  dispatch: dispatch2
}) {
  Object.defineProperties(this, {
    type: { value: type, enumerable: true, configurable: true },
    sourceEvent: { value: sourceEvent, enumerable: true, configurable: true },
    target: { value: target, enumerable: true, configurable: true },
    selection: { value: selection2, enumerable: true, configurable: true },
    mode: { value: mode2, enumerable: true, configurable: true },
    _: { value: dispatch2 }
  });
}

// node_modules/d3-brush/src/noevent.js
function nopropagation2(event) {
  event.stopImmediatePropagation();
}
function noevent_default2(event) {
  event.preventDefault();
  event.stopImmediatePropagation();
}

// node_modules/d3-brush/src/brush.js
var MODE_DRAG = { name: "drag" };
var MODE_SPACE = { name: "space" };
var MODE_HANDLE = { name: "handle" };
var MODE_CENTER = { name: "center" };
var { abs, max: max2, min: min2 } = Math;
function number1(e) {
  return [+e[0], +e[1]];
}
function number22(e) {
  return [number1(e[0]), number1(e[1])];
}
var X = {
  name: "x",
  handles: ["w", "e"].map(type),
  input: function(x, e) {
    return x == null ? null : [[+x[0], e[0][1]], [+x[1], e[1][1]]];
  },
  output: function(xy) {
    return xy && [xy[0][0], xy[1][0]];
  }
};
var Y = {
  name: "y",
  handles: ["n", "s"].map(type),
  input: function(y, e) {
    return y == null ? null : [[e[0][0], +y[0]], [e[1][0], +y[1]]];
  },
  output: function(xy) {
    return xy && [xy[0][1], xy[1][1]];
  }
};
var XY = {
  name: "xy",
  handles: ["n", "w", "e", "s", "nw", "ne", "sw", "se"].map(type),
  input: function(xy) {
    return xy == null ? null : number22(xy);
  },
  output: function(xy) {
    return xy;
  }
};
var cursors = {
  overlay: "crosshair",
  selection: "move",
  n: "ns-resize",
  e: "ew-resize",
  s: "ns-resize",
  w: "ew-resize",
  nw: "nwse-resize",
  ne: "nesw-resize",
  se: "nwse-resize",
  sw: "nesw-resize"
};
var flipX = {
  e: "w",
  w: "e",
  nw: "ne",
  ne: "nw",
  se: "sw",
  sw: "se"
};
var flipY = {
  n: "s",
  s: "n",
  nw: "sw",
  ne: "se",
  se: "ne",
  sw: "nw"
};
var signsX = {
  overlay: 1,
  selection: 1,
  n: null,
  e: 1,
  s: null,
  w: -1,
  nw: -1,
  ne: 1,
  se: 1,
  sw: -1
};
var signsY = {
  overlay: 1,
  selection: 1,
  n: -1,
  e: null,
  s: 1,
  w: null,
  nw: -1,
  ne: -1,
  se: 1,
  sw: 1
};
function type(t) {
  return { type: t };
}
function defaultFilter2(event) {
  return !event.ctrlKey && !event.button;
}
function defaultExtent() {
  var svg = this.ownerSVGElement || this;
  if (svg.hasAttribute("viewBox")) {
    svg = svg.viewBox.baseVal;
    return [[svg.x, svg.y], [svg.x + svg.width, svg.y + svg.height]];
  }
  return [[0, 0], [svg.width.baseVal.value, svg.height.baseVal.value]];
}
function defaultTouchable2() {
  return navigator.maxTouchPoints || "ontouchstart" in this;
}
function local2(node) {
  while (!node.__brush)
    if (!(node = node.parentNode))
      return;
  return node.__brush;
}
function empty3(extent2) {
  return extent2[0][0] === extent2[1][0] || extent2[0][1] === extent2[1][1];
}
function brushSelection(node) {
  var state = node.__brush;
  return state ? state.dim.output(state.selection) : null;
}
function brushX() {
  return brush(X);
}
function brushY() {
  return brush(Y);
}
function brush_default() {
  return brush(XY);
}
function brush(dim) {
  var extent2 = defaultExtent, filter3 = defaultFilter2, touchable = defaultTouchable2, keys = true, listeners = dispatch_default("start", "brush", "end"), handleSize = 6, touchending;
  function brush2(group2) {
    var overlay = group2.property("__brush", initialize).selectAll(".overlay").data([type("overlay")]);
    overlay.enter().append("rect").attr("class", "overlay").attr("pointer-events", "all").attr("cursor", cursors.overlay).merge(overlay).each(function() {
      var extent3 = local2(this).extent;
      select_default2(this).attr("x", extent3[0][0]).attr("y", extent3[0][1]).attr("width", extent3[1][0] - extent3[0][0]).attr("height", extent3[1][1] - extent3[0][1]);
    });
    group2.selectAll(".selection").data([type("selection")]).enter().append("rect").attr("class", "selection").attr("cursor", cursors.selection).attr("fill", "#777").attr("fill-opacity", 0.3).attr("stroke", "#fff").attr("shape-rendering", "crispEdges");
    var handle = group2.selectAll(".handle").data(dim.handles, function(d) {
      return d.type;
    });
    handle.exit().remove();
    handle.enter().append("rect").attr("class", function(d) {
      return "handle handle--" + d.type;
    }).attr("cursor", function(d) {
      return cursors[d.type];
    });
    group2.each(redraw).attr("fill", "none").attr("pointer-events", "all").on("mousedown.brush", started).filter(touchable).on("touchstart.brush", started).on("touchmove.brush", touchmoved).on("touchend.brush touchcancel.brush", touchended).style("touch-action", "none").style("-webkit-tap-highlight-color", "rgba(0,0,0,0)");
  }
  brush2.move = function(group2, selection2, event) {
    if (group2.tween) {
      group2.on("start.brush", function(event2) {
        emitter(this, arguments).beforestart().start(event2);
      }).on("interrupt.brush end.brush", function(event2) {
        emitter(this, arguments).end(event2);
      }).tween("brush", function() {
        var that = this, state = that.__brush, emit = emitter(that, arguments), selection0 = state.selection, selection1 = dim.input(typeof selection2 === "function" ? selection2.apply(this, arguments) : selection2, state.extent), i = value_default(selection0, selection1);
        function tween(t) {
          state.selection = t === 1 && selection1 === null ? null : i(t);
          redraw.call(that);
          emit.brush();
        }
        return selection0 !== null && selection1 !== null ? tween : tween(1);
      });
    } else {
      group2.each(function() {
        var that = this, args = arguments, state = that.__brush, selection1 = dim.input(typeof selection2 === "function" ? selection2.apply(that, args) : selection2, state.extent), emit = emitter(that, args).beforestart();
        interrupt_default(that);
        state.selection = selection1 === null ? null : selection1;
        redraw.call(that);
        emit.start(event).brush(event).end(event);
      });
    }
  };
  brush2.clear = function(group2, event) {
    brush2.move(group2, null, event);
  };
  function redraw() {
    var group2 = select_default2(this), selection2 = local2(this).selection;
    if (selection2) {
      group2.selectAll(".selection").style("display", null).attr("x", selection2[0][0]).attr("y", selection2[0][1]).attr("width", selection2[1][0] - selection2[0][0]).attr("height", selection2[1][1] - selection2[0][1]);
      group2.selectAll(".handle").style("display", null).attr("x", function(d) {
        return d.type[d.type.length - 1] === "e" ? selection2[1][0] - handleSize / 2 : selection2[0][0] - handleSize / 2;
      }).attr("y", function(d) {
        return d.type[0] === "s" ? selection2[1][1] - handleSize / 2 : selection2[0][1] - handleSize / 2;
      }).attr("width", function(d) {
        return d.type === "n" || d.type === "s" ? selection2[1][0] - selection2[0][0] + handleSize : handleSize;
      }).attr("height", function(d) {
        return d.type === "e" || d.type === "w" ? selection2[1][1] - selection2[0][1] + handleSize : handleSize;
      });
    } else {
      group2.selectAll(".selection,.handle").style("display", "none").attr("x", null).attr("y", null).attr("width", null).attr("height", null);
    }
  }
  function emitter(that, args, clean) {
    var emit = that.__brush.emitter;
    return emit && (!clean || !emit.clean) ? emit : new Emitter(that, args, clean);
  }
  function Emitter(that, args, clean) {
    this.that = that;
    this.args = args;
    this.state = that.__brush;
    this.active = 0;
    this.clean = clean;
  }
  Emitter.prototype = {
    beforestart: function() {
      if (++this.active === 1)
        this.state.emitter = this, this.starting = true;
      return this;
    },
    start: function(event, mode2) {
      if (this.starting)
        this.starting = false, this.emit("start", event, mode2);
      else
        this.emit("brush", event);
      return this;
    },
    brush: function(event, mode2) {
      this.emit("brush", event, mode2);
      return this;
    },
    end: function(event, mode2) {
      if (--this.active === 0)
        delete this.state.emitter, this.emit("end", event, mode2);
      return this;
    },
    emit: function(type2, event, mode2) {
      var d = select_default2(this.that).datum();
      listeners.call(type2, this.that, new BrushEvent(type2, {
        sourceEvent: event,
        target: brush2,
        selection: dim.output(this.state.selection),
        mode: mode2,
        dispatch: listeners
      }), d);
    }
  };
  function started(event) {
    if (touchending && !event.touches)
      return;
    if (!filter3.apply(this, arguments))
      return;
    var that = this, type2 = event.target.__data__.type, mode2 = (keys && event.metaKey ? type2 = "overlay" : type2) === "selection" ? MODE_DRAG : keys && event.altKey ? MODE_CENTER : MODE_HANDLE, signX = dim === Y ? null : signsX[type2], signY = dim === X ? null : signsY[type2], state = local2(that), extent3 = state.extent, selection2 = state.selection, W = extent3[0][0], w0, w1, N = extent3[0][1], n0, n1, E2 = extent3[1][0], e0, e1, S = extent3[1][1], s0, s1, dx = 0, dy = 0, moving, shifting = signX && signY && keys && event.shiftKey, lockX, lockY, points = Array.from(event.touches || [event], (t) => {
      const i = t.identifier;
      t = pointer_default(t, that);
      t.point0 = t.slice();
      t.identifier = i;
      return t;
    });
    interrupt_default(that);
    var emit = emitter(that, arguments, true).beforestart();
    if (type2 === "overlay") {
      if (selection2)
        moving = true;
      const pts = [points[0], points[1] || points[0]];
      state.selection = selection2 = [[
        w0 = dim === Y ? W : min2(pts[0][0], pts[1][0]),
        n0 = dim === X ? N : min2(pts[0][1], pts[1][1])
      ], [
        e0 = dim === Y ? E2 : max2(pts[0][0], pts[1][0]),
        s0 = dim === X ? S : max2(pts[0][1], pts[1][1])
      ]];
      if (points.length > 1)
        move(event);
    } else {
      w0 = selection2[0][0];
      n0 = selection2[0][1];
      e0 = selection2[1][0];
      s0 = selection2[1][1];
    }
    w1 = w0;
    n1 = n0;
    e1 = e0;
    s1 = s0;
    var group2 = select_default2(that).attr("pointer-events", "none");
    var overlay = group2.selectAll(".overlay").attr("cursor", cursors[type2]);
    if (event.touches) {
      emit.moved = moved;
      emit.ended = ended;
    } else {
      var view = select_default2(event.view).on("mousemove.brush", moved, true).on("mouseup.brush", ended, true);
      if (keys)
        view.on("keydown.brush", keydowned, true).on("keyup.brush", keyupped, true);
      nodrag_default(event.view);
    }
    redraw.call(that);
    emit.start(event, mode2.name);
    function moved(event2) {
      for (const p of event2.changedTouches || [event2]) {
        for (const d of points)
          if (d.identifier === p.identifier)
            d.cur = pointer_default(p, that);
      }
      if (shifting && !lockX && !lockY && points.length === 1) {
        const point = points[0];
        if (abs(point.cur[0] - point[0]) > abs(point.cur[1] - point[1]))
          lockY = true;
        else
          lockX = true;
      }
      for (const point of points)
        if (point.cur)
          point[0] = point.cur[0], point[1] = point.cur[1];
      moving = true;
      noevent_default2(event2);
      move(event2);
    }
    function move(event2) {
      const point = points[0], point0 = point.point0;
      var t;
      dx = point[0] - point0[0];
      dy = point[1] - point0[1];
      switch (mode2) {
        case MODE_SPACE:
        case MODE_DRAG: {
          if (signX)
            dx = max2(W - w0, min2(E2 - e0, dx)), w1 = w0 + dx, e1 = e0 + dx;
          if (signY)
            dy = max2(N - n0, min2(S - s0, dy)), n1 = n0 + dy, s1 = s0 + dy;
          break;
        }
        case MODE_HANDLE: {
          if (points[1]) {
            if (signX)
              w1 = max2(W, min2(E2, points[0][0])), e1 = max2(W, min2(E2, points[1][0])), signX = 1;
            if (signY)
              n1 = max2(N, min2(S, points[0][1])), s1 = max2(N, min2(S, points[1][1])), signY = 1;
          } else {
            if (signX < 0)
              dx = max2(W - w0, min2(E2 - w0, dx)), w1 = w0 + dx, e1 = e0;
            else if (signX > 0)
              dx = max2(W - e0, min2(E2 - e0, dx)), w1 = w0, e1 = e0 + dx;
            if (signY < 0)
              dy = max2(N - n0, min2(S - n0, dy)), n1 = n0 + dy, s1 = s0;
            else if (signY > 0)
              dy = max2(N - s0, min2(S - s0, dy)), n1 = n0, s1 = s0 + dy;
          }
          break;
        }
        case MODE_CENTER: {
          if (signX)
            w1 = max2(W, min2(E2, w0 - dx * signX)), e1 = max2(W, min2(E2, e0 + dx * signX));
          if (signY)
            n1 = max2(N, min2(S, n0 - dy * signY)), s1 = max2(N, min2(S, s0 + dy * signY));
          break;
        }
      }
      if (e1 < w1) {
        signX *= -1;
        t = w0, w0 = e0, e0 = t;
        t = w1, w1 = e1, e1 = t;
        if (type2 in flipX)
          overlay.attr("cursor", cursors[type2 = flipX[type2]]);
      }
      if (s1 < n1) {
        signY *= -1;
        t = n0, n0 = s0, s0 = t;
        t = n1, n1 = s1, s1 = t;
        if (type2 in flipY)
          overlay.attr("cursor", cursors[type2 = flipY[type2]]);
      }
      if (state.selection)
        selection2 = state.selection;
      if (lockX)
        w1 = selection2[0][0], e1 = selection2[1][0];
      if (lockY)
        n1 = selection2[0][1], s1 = selection2[1][1];
      if (selection2[0][0] !== w1 || selection2[0][1] !== n1 || selection2[1][0] !== e1 || selection2[1][1] !== s1) {
        state.selection = [[w1, n1], [e1, s1]];
        redraw.call(that);
        emit.brush(event2, mode2.name);
      }
    }
    function ended(event2) {
      nopropagation2(event2);
      if (event2.touches) {
        if (event2.touches.length)
          return;
        if (touchending)
          clearTimeout(touchending);
        touchending = setTimeout(function() {
          touchending = null;
        }, 500);
      } else {
        yesdrag(event2.view, moving);
        view.on("keydown.brush keyup.brush mousemove.brush mouseup.brush", null);
      }
      group2.attr("pointer-events", "all");
      overlay.attr("cursor", cursors.overlay);
      if (state.selection)
        selection2 = state.selection;
      if (empty3(selection2))
        state.selection = null, redraw.call(that);
      emit.end(event2, mode2.name);
    }
    function keydowned(event2) {
      switch (event2.keyCode) {
        case 16: {
          shifting = signX && signY;
          break;
        }
        case 18: {
          if (mode2 === MODE_HANDLE) {
            if (signX)
              e0 = e1 - dx * signX, w0 = w1 + dx * signX;
            if (signY)
              s0 = s1 - dy * signY, n0 = n1 + dy * signY;
            mode2 = MODE_CENTER;
            move(event2);
          }
          break;
        }
        case 32: {
          if (mode2 === MODE_HANDLE || mode2 === MODE_CENTER) {
            if (signX < 0)
              e0 = e1 - dx;
            else if (signX > 0)
              w0 = w1 - dx;
            if (signY < 0)
              s0 = s1 - dy;
            else if (signY > 0)
              n0 = n1 - dy;
            mode2 = MODE_SPACE;
            overlay.attr("cursor", cursors.selection);
            move(event2);
          }
          break;
        }
        default:
          return;
      }
      noevent_default2(event2);
    }
    function keyupped(event2) {
      switch (event2.keyCode) {
        case 16: {
          if (shifting) {
            lockX = lockY = shifting = false;
            move(event2);
          }
          break;
        }
        case 18: {
          if (mode2 === MODE_CENTER) {
            if (signX < 0)
              e0 = e1;
            else if (signX > 0)
              w0 = w1;
            if (signY < 0)
              s0 = s1;
            else if (signY > 0)
              n0 = n1;
            mode2 = MODE_HANDLE;
            move(event2);
          }
          break;
        }
        case 32: {
          if (mode2 === MODE_SPACE) {
            if (event2.altKey) {
              if (signX)
                e0 = e1 - dx * signX, w0 = w1 + dx * signX;
              if (signY)
                s0 = s1 - dy * signY, n0 = n1 + dy * signY;
              mode2 = MODE_CENTER;
            } else {
              if (signX < 0)
                e0 = e1;
              else if (signX > 0)
                w0 = w1;
              if (signY < 0)
                s0 = s1;
              else if (signY > 0)
                n0 = n1;
              mode2 = MODE_HANDLE;
            }
            overlay.attr("cursor", cursors[type2]);
            move(event2);
          }
          break;
        }
        default:
          return;
      }
      noevent_default2(event2);
    }
  }
  function touchmoved(event) {
    emitter(this, arguments).moved(event);
  }
  function touchended(event) {
    emitter(this, arguments).ended(event);
  }
  function initialize() {
    var state = this.__brush || { selection: null };
    state.extent = number22(extent2.apply(this, arguments));
    state.dim = dim;
    return state;
  }
  brush2.extent = function(_) {
    return arguments.length ? (extent2 = typeof _ === "function" ? _ : constant_default4(number22(_)), brush2) : extent2;
  };
  brush2.filter = function(_) {
    return arguments.length ? (filter3 = typeof _ === "function" ? _ : constant_default4(!!_), brush2) : filter3;
  };
  brush2.touchable = function(_) {
    return arguments.length ? (touchable = typeof _ === "function" ? _ : constant_default4(!!_), brush2) : touchable;
  };
  brush2.handleSize = function(_) {
    return arguments.length ? (handleSize = +_, brush2) : handleSize;
  };
  brush2.keyModifiers = function(_) {
    return arguments.length ? (keys = !!_, brush2) : keys;
  };
  brush2.on = function() {
    var value = listeners.on.apply(listeners, arguments);
    return value === listeners ? brush2 : value;
  };
  return brush2;
}
// node_modules/d3-chord/src/math.js
var abs2 = Math.abs;
var cos = Math.cos;
var sin = Math.sin;
var pi2 = Math.PI;
var halfPi2 = pi2 / 2;
var tau2 = pi2 * 2;
var max3 = Math.max;
var epsilon3 = 0.000000000001;

// node_modules/d3-chord/src/chord.js
function range2(i, j) {
  return Array.from({ length: j - i }, (_, k) => i + k);
}
function compareValue(compare) {
  return function(a, b) {
    return compare(a.source.value + a.target.value, b.source.value + b.target.value);
  };
}
function chord_default() {
  return chord(false, false);
}
function chordTranspose() {
  return chord(false, true);
}
function chordDirected() {
  return chord(true, false);
}
function chord(directed, transpose2) {
  var padAngle = 0, sortGroups = null, sortSubgroups = null, sortChords = null;
  function chord2(matrix) {
    var n = matrix.length, groupSums = new Array(n), groupIndex = range2(0, n), chords = new Array(n * n), groups2 = new Array(n), k = 0, dx;
    matrix = Float64Array.from({ length: n * n }, transpose2 ? (_, i) => matrix[i % n][i / n | 0] : (_, i) => matrix[i / n | 0][i % n]);
    for (let i = 0;i < n; ++i) {
      let x = 0;
      for (let j = 0;j < n; ++j)
        x += matrix[i * n + j] + directed * matrix[j * n + i];
      k += groupSums[i] = x;
    }
    k = max3(0, tau2 - padAngle * n) / k;
    dx = k ? padAngle : tau2 / n;
    {
      let x = 0;
      if (sortGroups)
        groupIndex.sort((a, b) => sortGroups(groupSums[a], groupSums[b]));
      for (const i of groupIndex) {
        const x0 = x;
        if (directed) {
          const subgroupIndex = range2(~n + 1, n).filter((j) => j < 0 ? matrix[~j * n + i] : matrix[i * n + j]);
          if (sortSubgroups)
            subgroupIndex.sort((a, b) => sortSubgroups(a < 0 ? -matrix[~a * n + i] : matrix[i * n + a], b < 0 ? -matrix[~b * n + i] : matrix[i * n + b]));
          for (const j of subgroupIndex) {
            if (j < 0) {
              const chord3 = chords[~j * n + i] || (chords[~j * n + i] = { source: null, target: null });
              chord3.target = { index: i, startAngle: x, endAngle: x += matrix[~j * n + i] * k, value: matrix[~j * n + i] };
            } else {
              const chord3 = chords[i * n + j] || (chords[i * n + j] = { source: null, target: null });
              chord3.source = { index: i, startAngle: x, endAngle: x += matrix[i * n + j] * k, value: matrix[i * n + j] };
            }
          }
          groups2[i] = { index: i, startAngle: x0, endAngle: x, value: groupSums[i] };
        } else {
          const subgroupIndex = range2(0, n).filter((j) => matrix[i * n + j] || matrix[j * n + i]);
          if (sortSubgroups)
            subgroupIndex.sort((a, b) => sortSubgroups(matrix[i * n + a], matrix[i * n + b]));
          for (const j of subgroupIndex) {
            let chord3;
            if (i < j) {
              chord3 = chords[i * n + j] || (chords[i * n + j] = { source: null, target: null });
              chord3.source = { index: i, startAngle: x, endAngle: x += matrix[i * n + j] * k, value: matrix[i * n + j] };
            } else {
              chord3 = chords[j * n + i] || (chords[j * n + i] = { source: null, target: null });
              chord3.target = { index: i, startAngle: x, endAngle: x += matrix[i * n + j] * k, value: matrix[i * n + j] };
              if (i === j)
                chord3.source = chord3.target;
            }
            if (chord3.source && chord3.target && chord3.source.value < chord3.target.value) {
              const source = chord3.source;
              chord3.source = chord3.target;
              chord3.target = source;
            }
          }
          groups2[i] = { index: i, startAngle: x0, endAngle: x, value: groupSums[i] };
        }
        x += dx;
      }
    }
    chords = Object.values(chords);
    chords.groups = groups2;
    return sortChords ? chords.sort(sortChords) : chords;
  }
  chord2.padAngle = function(_) {
    return arguments.length ? (padAngle = max3(0, _), chord2) : padAngle;
  };
  chord2.sortGroups = function(_) {
    return arguments.length ? (sortGroups = _, chord2) : sortGroups;
  };
  chord2.sortSubgroups = function(_) {
    return arguments.length ? (sortSubgroups = _, chord2) : sortSubgroups;
  };
  chord2.sortChords = function(_) {
    return arguments.length ? (_ == null ? sortChords = null : (sortChords = compareValue(_))._ = _, chord2) : sortChords && sortChords._;
  };
  return chord2;
}
// node_modules/d3-path/src/path.js
var pi3 = Math.PI;
var tau3 = 2 * pi3;
var epsilon5 = 0.000001;
var tauEpsilon = tau3 - epsilon5;
function append(strings) {
  this._ += strings[0];
  for (let i = 1, n = strings.length;i < n; ++i) {
    this._ += arguments[i] + strings[i];
  }
}
function appendRound(digits) {
  let d = Math.floor(digits);
  if (!(d >= 0))
    throw new Error(`invalid digits: ${digits}`);
  if (d > 15)
    return append;
  const k = 10 ** d;
  return function(strings) {
    this._ += strings[0];
    for (let i = 1, n = strings.length;i < n; ++i) {
      this._ += Math.round(arguments[i] * k) / k + strings[i];
    }
  };
}

class Path {
  constructor(digits) {
    this._x0 = this._y0 = this._x1 = this._y1 = null;
    this._ = "";
    this._append = digits == null ? append : appendRound(digits);
  }
  moveTo(x, y) {
    this._append`M${this._x0 = this._x1 = +x},${this._y0 = this._y1 = +y}`;
  }
  closePath() {
    if (this._x1 !== null) {
      this._x1 = this._x0, this._y1 = this._y0;
      this._append`Z`;
    }
  }
  lineTo(x, y) {
    this._append`L${this._x1 = +x},${this._y1 = +y}`;
  }
  quadraticCurveTo(x1, y1, x, y) {
    this._append`Q${+x1},${+y1},${this._x1 = +x},${this._y1 = +y}`;
  }
  bezierCurveTo(x1, y1, x2, y2, x, y) {
    this._append`C${+x1},${+y1},${+x2},${+y2},${this._x1 = +x},${this._y1 = +y}`;
  }
  arcTo(x1, y1, x2, y2, r) {
    x1 = +x1, y1 = +y1, x2 = +x2, y2 = +y2, r = +r;
    if (r < 0)
      throw new Error(`negative radius: ${r}`);
    let x0 = this._x1, y0 = this._y1, x21 = x2 - x1, y21 = y2 - y1, x01 = x0 - x1, y01 = y0 - y1, l01_2 = x01 * x01 + y01 * y01;
    if (this._x1 === null) {
      this._append`M${this._x1 = x1},${this._y1 = y1}`;
    } else if (!(l01_2 > epsilon5))
      ;
    else if (!(Math.abs(y01 * x21 - y21 * x01) > epsilon5) || !r) {
      this._append`L${this._x1 = x1},${this._y1 = y1}`;
    } else {
      let x20 = x2 - x0, y20 = y2 - y0, l21_2 = x21 * x21 + y21 * y21, l20_2 = x20 * x20 + y20 * y20, l21 = Math.sqrt(l21_2), l01 = Math.sqrt(l01_2), l = r * Math.tan((pi3 - Math.acos((l21_2 + l01_2 - l20_2) / (2 * l21 * l01))) / 2), t01 = l / l01, t21 = l / l21;
      if (Math.abs(t01 - 1) > epsilon5) {
        this._append`L${x1 + t01 * x01},${y1 + t01 * y01}`;
      }
      this._append`A${r},${r},0,0,${+(y01 * x20 > x01 * y20)},${this._x1 = x1 + t21 * x21},${this._y1 = y1 + t21 * y21}`;
    }
  }
  arc(x, y, r, a0, a1, ccw) {
    x = +x, y = +y, r = +r, ccw = !!ccw;
    if (r < 0)
      throw new Error(`negative radius: ${r}`);
    let dx = r * Math.cos(a0), dy = r * Math.sin(a0), x0 = x + dx, y0 = y + dy, cw = 1 ^ ccw, da = ccw ? a0 - a1 : a1 - a0;
    if (this._x1 === null) {
      this._append`M${x0},${y0}`;
    } else if (Math.abs(this._x1 - x0) > epsilon5 || Math.abs(this._y1 - y0) > epsilon5) {
      this._append`L${x0},${y0}`;
    }
    if (!r)
      return;
    if (da < 0)
      da = da % tau3 + tau3;
    if (da > tauEpsilon) {
      this._append`A${r},${r},0,1,${cw},${x - dx},${y - dy}A${r},${r},0,1,${cw},${this._x1 = x0},${this._y1 = y0}`;
    } else if (da > epsilon5) {
      this._append`A${r},${r},0,${+(da >= pi3)},${cw},${this._x1 = x + r * Math.cos(a1)},${this._y1 = y + r * Math.sin(a1)}`;
    }
  }
  rect(x, y, w, h) {
    this._append`M${this._x0 = this._x1 = +x},${this._y0 = this._y1 = +y}h${w = +w}v${+h}h${-w}Z`;
  }
  toString() {
    return this._;
  }
}
function path() {
  return new Path;
}
path.prototype = Path.prototype;
function pathRound(digits = 3) {
  return new Path(+digits);
}
// node_modules/d3-chord/src/array.js
var slice2 = Array.prototype.slice;

// node_modules/d3-chord/src/constant.js
function constant_default5(x) {
  return function() {
    return x;
  };
}

// node_modules/d3-chord/src/ribbon.js
function defaultSource(d) {
  return d.source;
}
function defaultTarget(d) {
  return d.target;
}
function defaultRadius(d) {
  return d.radius;
}
function defaultStartAngle(d) {
  return d.startAngle;
}
function defaultEndAngle(d) {
  return d.endAngle;
}
function defaultPadAngle() {
  return 0;
}
function defaultArrowheadRadius() {
  return 10;
}
function ribbon(headRadius) {
  var source = defaultSource, target = defaultTarget, sourceRadius = defaultRadius, targetRadius = defaultRadius, startAngle = defaultStartAngle, endAngle = defaultEndAngle, padAngle = defaultPadAngle, context = null;
  function ribbon2() {
    var buffer, s = source.apply(this, arguments), t = target.apply(this, arguments), ap = padAngle.apply(this, arguments) / 2, argv = slice2.call(arguments), sr = +sourceRadius.apply(this, (argv[0] = s, argv)), sa0 = startAngle.apply(this, argv) - halfPi2, sa1 = endAngle.apply(this, argv) - halfPi2, tr = +targetRadius.apply(this, (argv[0] = t, argv)), ta0 = startAngle.apply(this, argv) - halfPi2, ta1 = endAngle.apply(this, argv) - halfPi2;
    if (!context)
      context = buffer = path();
    if (ap > epsilon3) {
      if (abs2(sa1 - sa0) > ap * 2 + epsilon3)
        sa1 > sa0 ? (sa0 += ap, sa1 -= ap) : (sa0 -= ap, sa1 += ap);
      else
        sa0 = sa1 = (sa0 + sa1) / 2;
      if (abs2(ta1 - ta0) > ap * 2 + epsilon3)
        ta1 > ta0 ? (ta0 += ap, ta1 -= ap) : (ta0 -= ap, ta1 += ap);
      else
        ta0 = ta1 = (ta0 + ta1) / 2;
    }
    context.moveTo(sr * cos(sa0), sr * sin(sa0));
    context.arc(0, 0, sr, sa0, sa1);
    if (sa0 !== ta0 || sa1 !== ta1) {
      if (headRadius) {
        var hr = +headRadius.apply(this, arguments), tr2 = tr - hr, ta2 = (ta0 + ta1) / 2;
        context.quadraticCurveTo(0, 0, tr2 * cos(ta0), tr2 * sin(ta0));
        context.lineTo(tr * cos(ta2), tr * sin(ta2));
        context.lineTo(tr2 * cos(ta1), tr2 * sin(ta1));
      } else {
        context.quadraticCurveTo(0, 0, tr * cos(ta0), tr * sin(ta0));
        context.arc(0, 0, tr, ta0, ta1);
      }
    }
    context.quadraticCurveTo(0, 0, sr * cos(sa0), sr * sin(sa0));
    context.closePath();
    if (buffer)
      return context = null, buffer + "" || null;
  }
  if (headRadius)
    ribbon2.headRadius = function(_) {
      return arguments.length ? (headRadius = typeof _ === "function" ? _ : constant_default5(+_), ribbon2) : headRadius;
    };
  ribbon2.radius = function(_) {
    return arguments.length ? (sourceRadius = targetRadius = typeof _ === "function" ? _ : constant_default5(+_), ribbon2) : sourceRadius;
  };
  ribbon2.sourceRadius = function(_) {
    return arguments.length ? (sourceRadius = typeof _ === "function" ? _ : constant_default5(+_), ribbon2) : sourceRadius;
  };
  ribbon2.targetRadius = function(_) {
    return arguments.length ? (targetRadius = typeof _ === "function" ? _ : constant_default5(+_), ribbon2) : targetRadius;
  };
  ribbon2.startAngle = function(_) {
    return arguments.length ? (startAngle = typeof _ === "function" ? _ : constant_default5(+_), ribbon2) : startAngle;
  };
  ribbon2.endAngle = function(_) {
    return arguments.length ? (endAngle = typeof _ === "function" ? _ : constant_default5(+_), ribbon2) : endAngle;
  };
  ribbon2.padAngle = function(_) {
    return arguments.length ? (padAngle = typeof _ === "function" ? _ : constant_default5(+_), ribbon2) : padAngle;
  };
  ribbon2.source = function(_) {
    return arguments.length ? (source = _, ribbon2) : source;
  };
  ribbon2.target = function(_) {
    return arguments.length ? (target = _, ribbon2) : target;
  };
  ribbon2.context = function(_) {
    return arguments.length ? (context = _ == null ? null : _, ribbon2) : context;
  };
  return ribbon2;
}
function ribbon_default() {
  return ribbon();
}
function ribbonArrow() {
  return ribbon(defaultArrowheadRadius);
}
// node_modules/d3-contour/src/array.js
var array3 = Array.prototype;
var slice3 = array3.slice;

// node_modules/d3-contour/src/ascending.js
function ascending_default(a, b) {
  return a - b;
}

// node_modules/d3-contour/src/area.js
function area_default(ring) {
  var i = 0, n = ring.length, area = ring[n - 1][1] * ring[0][0] - ring[n - 1][0] * ring[0][1];
  while (++i < n)
    area += ring[i - 1][1] * ring[i][0] - ring[i - 1][0] * ring[i][1];
  return area;
}

// node_modules/d3-contour/src/constant.js
var constant_default6 = (x) => () => x;

// node_modules/d3-contour/src/contains.js
function contains_default(ring, hole) {
  var i = -1, n = hole.length, c;
  while (++i < n)
    if (c = ringContains(ring, hole[i]))
      return c;
  return 0;
}
function ringContains(ring, point) {
  var x = point[0], y = point[1], contains = -1;
  for (var i = 0, n = ring.length, j = n - 1;i < n; j = i++) {
    var pi4 = ring[i], xi = pi4[0], yi = pi4[1], pj = ring[j], xj = pj[0], yj = pj[1];
    if (segmentContains(pi4, pj, point))
      return 0;
    if (yi > y !== yj > y && x < (xj - xi) * (y - yi) / (yj - yi) + xi)
      contains = -contains;
  }
  return contains;
}
function segmentContains(a, b, c) {
  var i;
  return collinear(a, b, c) && within(a[i = +(a[0] === b[0])], c[i], b[i]);
}
function collinear(a, b, c) {
  return (b[0] - a[0]) * (c[1] - a[1]) === (c[0] - a[0]) * (b[1] - a[1]);
}
function within(p, q, r) {
  return p <= q && q <= r || r <= q && q <= p;
}

// node_modules/d3-contour/src/noop.js
function noop_default() {}

// node_modules/d3-contour/src/contours.js
var cases = [
  [],
  [[[1, 1.5], [0.5, 1]]],
  [[[1.5, 1], [1, 1.5]]],
  [[[1.5, 1], [0.5, 1]]],
  [[[1, 0.5], [1.5, 1]]],
  [[[1, 1.5], [0.5, 1]], [[1, 0.5], [1.5, 1]]],
  [[[1, 0.5], [1, 1.5]]],
  [[[1, 0.5], [0.5, 1]]],
  [[[0.5, 1], [1, 0.5]]],
  [[[1, 1.5], [1, 0.5]]],
  [[[0.5, 1], [1, 0.5]], [[1.5, 1], [1, 1.5]]],
  [[[1.5, 1], [1, 0.5]]],
  [[[0.5, 1], [1.5, 1]]],
  [[[1, 1.5], [1.5, 1]]],
  [[[0.5, 1], [1, 1.5]]],
  []
];
function contours_default() {
  var dx = 1, dy = 1, threshold = thresholdSturges, smooth = smoothLinear;
  function contours(values) {
    var tz = threshold(values);
    if (!Array.isArray(tz)) {
      const e = extent(values, finite);
      tz = ticks(...nice(e[0], e[1], tz), tz);
      while (tz[tz.length - 1] >= e[1])
        tz.pop();
      while (tz[1] < e[0])
        tz.shift();
    } else {
      tz = tz.slice().sort(ascending_default);
    }
    return tz.map((value) => contour(values, value));
  }
  function contour(values, value) {
    const v = value == null ? NaN : +value;
    if (isNaN(v))
      throw new Error(`invalid value: ${value}`);
    var polygons = [], holes = [];
    isorings(values, v, function(ring) {
      smooth(ring, values, v);
      if (area_default(ring) > 0)
        polygons.push([ring]);
      else
        holes.push(ring);
    });
    holes.forEach(function(hole) {
      for (var i = 0, n = polygons.length, polygon;i < n; ++i) {
        if (contains_default((polygon = polygons[i])[0], hole) !== -1) {
          polygon.push(hole);
          return;
        }
      }
    });
    return {
      type: "MultiPolygon",
      value,
      coordinates: polygons
    };
  }
  function isorings(values, value, callback) {
    var fragmentByStart = new Array, fragmentByEnd = new Array, x, y, t02, t12, t22, t32;
    x = y = -1;
    t12 = above(values[0], value);
    cases[t12 << 1].forEach(stitch);
    while (++x < dx - 1) {
      t02 = t12, t12 = above(values[x + 1], value);
      cases[t02 | t12 << 1].forEach(stitch);
    }
    cases[t12 << 0].forEach(stitch);
    while (++y < dy - 1) {
      x = -1;
      t12 = above(values[y * dx + dx], value);
      t22 = above(values[y * dx], value);
      cases[t12 << 1 | t22 << 2].forEach(stitch);
      while (++x < dx - 1) {
        t02 = t12, t12 = above(values[y * dx + dx + x + 1], value);
        t32 = t22, t22 = above(values[y * dx + x + 1], value);
        cases[t02 | t12 << 1 | t22 << 2 | t32 << 3].forEach(stitch);
      }
      cases[t12 | t22 << 3].forEach(stitch);
    }
    x = -1;
    t22 = values[y * dx] >= value;
    cases[t22 << 2].forEach(stitch);
    while (++x < dx - 1) {
      t32 = t22, t22 = above(values[y * dx + x + 1], value);
      cases[t22 << 2 | t32 << 3].forEach(stitch);
    }
    cases[t22 << 3].forEach(stitch);
    function stitch(line) {
      var start2 = [line[0][0] + x, line[0][1] + y], end = [line[1][0] + x, line[1][1] + y], startIndex = index2(start2), endIndex = index2(end), f, g;
      if (f = fragmentByEnd[startIndex]) {
        if (g = fragmentByStart[endIndex]) {
          delete fragmentByEnd[f.end];
          delete fragmentByStart[g.start];
          if (f === g) {
            f.ring.push(end);
            callback(f.ring);
          } else {
            fragmentByStart[f.start] = fragmentByEnd[g.end] = { start: f.start, end: g.end, ring: f.ring.concat(g.ring) };
          }
        } else {
          delete fragmentByEnd[f.end];
          f.ring.push(end);
          fragmentByEnd[f.end = endIndex] = f;
        }
      } else if (f = fragmentByStart[endIndex]) {
        if (g = fragmentByEnd[startIndex]) {
          delete fragmentByStart[f.start];
          delete fragmentByEnd[g.end];
          if (f === g) {
            f.ring.push(end);
            callback(f.ring);
          } else {
            fragmentByStart[g.start] = fragmentByEnd[f.end] = { start: g.start, end: f.end, ring: g.ring.concat(f.ring) };
          }
        } else {
          delete fragmentByStart[f.start];
          f.ring.unshift(start2);
          fragmentByStart[f.start = startIndex] = f;
        }
      } else {
        fragmentByStart[startIndex] = fragmentByEnd[endIndex] = { start: startIndex, end: endIndex, ring: [start2, end] };
      }
    }
  }
  function index2(point) {
    return point[0] * 2 + point[1] * (dx + 1) * 4;
  }
  function smoothLinear(ring, values, value) {
    ring.forEach(function(point) {
      var x = point[0], y = point[1], xt = x | 0, yt = y | 0, v1 = valid(values[yt * dx + xt]);
      if (x > 0 && x < dx && xt === x) {
        point[0] = smooth1(x, valid(values[yt * dx + xt - 1]), v1, value);
      }
      if (y > 0 && y < dy && yt === y) {
        point[1] = smooth1(y, valid(values[(yt - 1) * dx + xt]), v1, value);
      }
    });
  }
  contours.contour = contour;
  contours.size = function(_) {
    if (!arguments.length)
      return [dx, dy];
    var _0 = Math.floor(_[0]), _1 = Math.floor(_[1]);
    if (!(_0 >= 0 && _1 >= 0))
      throw new Error("invalid size");
    return dx = _0, dy = _1, contours;
  };
  contours.thresholds = function(_) {
    return arguments.length ? (threshold = typeof _ === "function" ? _ : Array.isArray(_) ? constant_default6(slice3.call(_)) : constant_default6(_), contours) : threshold;
  };
  contours.smooth = function(_) {
    return arguments.length ? (smooth = _ ? smoothLinear : noop_default, contours) : smooth === smoothLinear;
  };
  return contours;
}
function finite(x) {
  return isFinite(x) ? x : NaN;
}
function above(x, value) {
  return x == null ? false : +x >= value;
}
function valid(v) {
  return v == null || isNaN(v = +v) ? -Infinity : v;
}
function smooth1(x, v0, v1, value) {
  const a = value - v0;
  const b = v1 - v0;
  const d = isFinite(a) || isFinite(b) ? a / b : Math.sign(a) / Math.sign(b);
  return isNaN(d) ? x : x + d - 0.5;
}
// node_modules/d3-contour/src/density.js
function defaultX(d) {
  return d[0];
}
function defaultY(d) {
  return d[1];
}
function defaultWeight() {
  return 1;
}
function density_default() {
  var x = defaultX, y = defaultY, weight = defaultWeight, dx = 960, dy = 500, r = 20, k = 2, o = r * 3, n = dx + o * 2 >> k, m = dy + o * 2 >> k, threshold = constant_default6(20);
  function grid(data) {
    var values = new Float32Array(n * m), pow2k = Math.pow(2, -k), i = -1;
    for (const d of data) {
      var xi = (x(d, ++i, data) + o) * pow2k, yi = (y(d, i, data) + o) * pow2k, wi = +weight(d, i, data);
      if (wi && xi >= 0 && xi < n && yi >= 0 && yi < m) {
        var x0 = Math.floor(xi), y0 = Math.floor(yi), xt = xi - x0 - 0.5, yt = yi - y0 - 0.5;
        values[x0 + y0 * n] += (1 - xt) * (1 - yt) * wi;
        values[x0 + 1 + y0 * n] += xt * (1 - yt) * wi;
        values[x0 + 1 + (y0 + 1) * n] += xt * yt * wi;
        values[x0 + (y0 + 1) * n] += (1 - xt) * yt * wi;
      }
    }
    blur2({ data: values, width: n, height: m }, r * pow2k);
    return values;
  }
  function density(data) {
    var values = grid(data), tz = threshold(values), pow4k = Math.pow(2, 2 * k);
    if (!Array.isArray(tz)) {
      tz = ticks(Number.MIN_VALUE, max(values) / pow4k, tz);
    }
    return contours_default().size([n, m]).thresholds(tz.map((d) => d * pow4k))(values).map((c, i) => (c.value = +tz[i], transform(c)));
  }
  density.contours = function(data) {
    var values = grid(data), contours = contours_default().size([n, m]), pow4k = Math.pow(2, 2 * k), contour = (value) => {
      value = +value;
      var c = transform(contours.contour(values, value * pow4k));
      c.value = value;
      return c;
    };
    Object.defineProperty(contour, "max", { get: () => max(values) / pow4k });
    return contour;
  };
  function transform(geometry) {
    geometry.coordinates.forEach(transformPolygon);
    return geometry;
  }
  function transformPolygon(coordinates) {
    coordinates.forEach(transformRing);
  }
  function transformRing(coordinates) {
    coordinates.forEach(transformPoint);
  }
  function transformPoint(coordinates) {
    coordinates[0] = coordinates[0] * Math.pow(2, k) - o;
    coordinates[1] = coordinates[1] * Math.pow(2, k) - o;
  }
  function resize() {
    o = r * 3;
    n = dx + o * 2 >> k;
    m = dy + o * 2 >> k;
    return density;
  }
  density.x = function(_) {
    return arguments.length ? (x = typeof _ === "function" ? _ : constant_default6(+_), density) : x;
  };
  density.y = function(_) {
    return arguments.length ? (y = typeof _ === "function" ? _ : constant_default6(+_), density) : y;
  };
  density.weight = function(_) {
    return arguments.length ? (weight = typeof _ === "function" ? _ : constant_default6(+_), density) : weight;
  };
  density.size = function(_) {
    if (!arguments.length)
      return [dx, dy];
    var _0 = +_[0], _1 = +_[1];
    if (!(_0 >= 0 && _1 >= 0))
      throw new Error("invalid size");
    return dx = _0, dy = _1, resize();
  };
  density.cellSize = function(_) {
    if (!arguments.length)
      return 1 << k;
    if (!((_ = +_) >= 1))
      throw new Error("invalid cell size");
    return k = Math.floor(Math.log(_) / Math.LN2), resize();
  };
  density.thresholds = function(_) {
    return arguments.length ? (threshold = typeof _ === "function" ? _ : Array.isArray(_) ? constant_default6(slice3.call(_)) : constant_default6(_), density) : threshold;
  };
  density.bandwidth = function(_) {
    if (!arguments.length)
      return Math.sqrt(r * (r + 1));
    if (!((_ = +_) >= 0))
      throw new Error("invalid bandwidth");
    return r = (Math.sqrt(4 * _ * _ + 1) - 1) / 2, resize();
  };
  return density;
}
// node_modules/robust-predicates/esm/util.js
var epsilon6 = 0.00000000000000011102230246251565;
var splitter = 134217729;
var resulterrbound = (3 + 8 * epsilon6) * epsilon6;
function sum2(elen, e, flen, f, h) {
  let Q, Qnew, hh, bvirt;
  let enow = e[0];
  let fnow = f[0];
  let eindex = 0;
  let findex = 0;
  if (fnow > enow === fnow > -enow) {
    Q = enow;
    enow = e[++eindex];
  } else {
    Q = fnow;
    fnow = f[++findex];
  }
  let hindex = 0;
  if (eindex < elen && findex < flen) {
    if (fnow > enow === fnow > -enow) {
      Qnew = enow + Q;
      hh = Q - (Qnew - enow);
      enow = e[++eindex];
    } else {
      Qnew = fnow + Q;
      hh = Q - (Qnew - fnow);
      fnow = f[++findex];
    }
    Q = Qnew;
    if (hh !== 0) {
      h[hindex++] = hh;
    }
    while (eindex < elen && findex < flen) {
      if (fnow > enow === fnow > -enow) {
        Qnew = Q + enow;
        bvirt = Qnew - Q;
        hh = Q - (Qnew - bvirt) + (enow - bvirt);
        enow = e[++eindex];
      } else {
        Qnew = Q + fnow;
        bvirt = Qnew - Q;
        hh = Q - (Qnew - bvirt) + (fnow - bvirt);
        fnow = f[++findex];
      }
      Q = Qnew;
      if (hh !== 0) {
        h[hindex++] = hh;
      }
    }
  }
  while (eindex < elen) {
    Qnew = Q + enow;
    bvirt = Qnew - Q;
    hh = Q - (Qnew - bvirt) + (enow - bvirt);
    enow = e[++eindex];
    Q = Qnew;
    if (hh !== 0) {
      h[hindex++] = hh;
    }
  }
  while (findex < flen) {
    Qnew = Q + fnow;
    bvirt = Qnew - Q;
    hh = Q - (Qnew - bvirt) + (fnow - bvirt);
    fnow = f[++findex];
    Q = Qnew;
    if (hh !== 0) {
      h[hindex++] = hh;
    }
  }
  if (Q !== 0 || hindex === 0) {
    h[hindex++] = Q;
  }
  return hindex;
}
function estimate(elen, e) {
  let Q = e[0];
  for (let i = 1;i < elen; i++)
    Q += e[i];
  return Q;
}
function vec(n) {
  return new Float64Array(n);
}

// node_modules/robust-predicates/esm/orient2d.js
var ccwerrboundA = (3 + 16 * epsilon6) * epsilon6;
var ccwerrboundB = (2 + 12 * epsilon6) * epsilon6;
var ccwerrboundC = (9 + 64 * epsilon6) * epsilon6 * epsilon6;
var B2 = vec(4);
var C1 = vec(8);
var C2 = vec(12);
var D2 = vec(16);
var u = vec(4);
function orient2dadapt(ax, ay, bx, by, cx, cy, detsum) {
  let acxtail, acytail, bcxtail, bcytail;
  let bvirt, c, ahi, alo, bhi, blo, _i, _j, _0, s1, s0, t12, t02, u3;
  const acx = ax - cx;
  const bcx = bx - cx;
  const acy = ay - cy;
  const bcy = by - cy;
  s1 = acx * bcy;
  c = splitter * acx;
  ahi = c - (c - acx);
  alo = acx - ahi;
  c = splitter * bcy;
  bhi = c - (c - bcy);
  blo = bcy - bhi;
  s0 = alo * blo - (s1 - ahi * bhi - alo * bhi - ahi * blo);
  t12 = acy * bcx;
  c = splitter * acy;
  ahi = c - (c - acy);
  alo = acy - ahi;
  c = splitter * bcx;
  bhi = c - (c - bcx);
  blo = bcx - bhi;
  t02 = alo * blo - (t12 - ahi * bhi - alo * bhi - ahi * blo);
  _i = s0 - t02;
  bvirt = s0 - _i;
  B2[0] = s0 - (_i + bvirt) + (bvirt - t02);
  _j = s1 + _i;
  bvirt = _j - s1;
  _0 = s1 - (_j - bvirt) + (_i - bvirt);
  _i = _0 - t12;
  bvirt = _0 - _i;
  B2[1] = _0 - (_i + bvirt) + (bvirt - t12);
  u3 = _j + _i;
  bvirt = u3 - _j;
  B2[2] = _j - (u3 - bvirt) + (_i - bvirt);
  B2[3] = u3;
  let det = estimate(4, B2);
  let errbound = ccwerrboundB * detsum;
  if (det >= errbound || -det >= errbound) {
    return det;
  }
  bvirt = ax - acx;
  acxtail = ax - (acx + bvirt) + (bvirt - cx);
  bvirt = bx - bcx;
  bcxtail = bx - (bcx + bvirt) + (bvirt - cx);
  bvirt = ay - acy;
  acytail = ay - (acy + bvirt) + (bvirt - cy);
  bvirt = by - bcy;
  bcytail = by - (bcy + bvirt) + (bvirt - cy);
  if (acxtail === 0 && acytail === 0 && bcxtail === 0 && bcytail === 0) {
    return det;
  }
  errbound = ccwerrboundC * detsum + resulterrbound * Math.abs(det);
  det += acx * bcytail + bcy * acxtail - (acy * bcxtail + bcx * acytail);
  if (det >= errbound || -det >= errbound)
    return det;
  s1 = acxtail * bcy;
  c = splitter * acxtail;
  ahi = c - (c - acxtail);
  alo = acxtail - ahi;
  c = splitter * bcy;
  bhi = c - (c - bcy);
  blo = bcy - bhi;
  s0 = alo * blo - (s1 - ahi * bhi - alo * bhi - ahi * blo);
  t12 = acytail * bcx;
  c = splitter * acytail;
  ahi = c - (c - acytail);
  alo = acytail - ahi;
  c = splitter * bcx;
  bhi = c - (c - bcx);
  blo = bcx - bhi;
  t02 = alo * blo - (t12 - ahi * bhi - alo * bhi - ahi * blo);
  _i = s0 - t02;
  bvirt = s0 - _i;
  u[0] = s0 - (_i + bvirt) + (bvirt - t02);
  _j = s1 + _i;
  bvirt = _j - s1;
  _0 = s1 - (_j - bvirt) + (_i - bvirt);
  _i = _0 - t12;
  bvirt = _0 - _i;
  u[1] = _0 - (_i + bvirt) + (bvirt - t12);
  u3 = _j + _i;
  bvirt = u3 - _j;
  u[2] = _j - (u3 - bvirt) + (_i - bvirt);
  u[3] = u3;
  const C1len = sum2(4, B2, 4, u, C1);
  s1 = acx * bcytail;
  c = splitter * acx;
  ahi = c - (c - acx);
  alo = acx - ahi;
  c = splitter * bcytail;
  bhi = c - (c - bcytail);
  blo = bcytail - bhi;
  s0 = alo * blo - (s1 - ahi * bhi - alo * bhi - ahi * blo);
  t12 = acy * bcxtail;
  c = splitter * acy;
  ahi = c - (c - acy);
  alo = acy - ahi;
  c = splitter * bcxtail;
  bhi = c - (c - bcxtail);
  blo = bcxtail - bhi;
  t02 = alo * blo - (t12 - ahi * bhi - alo * bhi - ahi * blo);
  _i = s0 - t02;
  bvirt = s0 - _i;
  u[0] = s0 - (_i + bvirt) + (bvirt - t02);
  _j = s1 + _i;
  bvirt = _j - s1;
  _0 = s1 - (_j - bvirt) + (_i - bvirt);
  _i = _0 - t12;
  bvirt = _0 - _i;
  u[1] = _0 - (_i + bvirt) + (bvirt - t12);
  u3 = _j + _i;
  bvirt = u3 - _j;
  u[2] = _j - (u3 - bvirt) + (_i - bvirt);
  u[3] = u3;
  const C2len = sum2(C1len, C1, 4, u, C2);
  s1 = acxtail * bcytail;
  c = splitter * acxtail;
  ahi = c - (c - acxtail);
  alo = acxtail - ahi;
  c = splitter * bcytail;
  bhi = c - (c - bcytail);
  blo = bcytail - bhi;
  s0 = alo * blo - (s1 - ahi * bhi - alo * bhi - ahi * blo);
  t12 = acytail * bcxtail;
  c = splitter * acytail;
  ahi = c - (c - acytail);
  alo = acytail - ahi;
  c = splitter * bcxtail;
  bhi = c - (c - bcxtail);
  blo = bcxtail - bhi;
  t02 = alo * blo - (t12 - ahi * bhi - alo * bhi - ahi * blo);
  _i = s0 - t02;
  bvirt = s0 - _i;
  u[0] = s0 - (_i + bvirt) + (bvirt - t02);
  _j = s1 + _i;
  bvirt = _j - s1;
  _0 = s1 - (_j - bvirt) + (_i - bvirt);
  _i = _0 - t12;
  bvirt = _0 - _i;
  u[1] = _0 - (_i + bvirt) + (bvirt - t12);
  u3 = _j + _i;
  bvirt = u3 - _j;
  u[2] = _j - (u3 - bvirt) + (_i - bvirt);
  u[3] = u3;
  const Dlen = sum2(C2len, C2, 4, u, D2);
  return D2[Dlen - 1];
}
function orient2d(ax, ay, bx, by, cx, cy) {
  const detleft = (ay - cy) * (bx - cx);
  const detright = (ax - cx) * (by - cy);
  const det = detleft - detright;
  const detsum = Math.abs(detleft + detright);
  if (Math.abs(det) >= ccwerrboundA * detsum)
    return det;
  return -orient2dadapt(ax, ay, bx, by, cx, cy, detsum);
}
// node_modules/robust-predicates/esm/orient3d.js
var o3derrboundA = (7 + 56 * epsilon6) * epsilon6;
var o3derrboundB = (3 + 28 * epsilon6) * epsilon6;
var o3derrboundC = (26 + 288 * epsilon6) * epsilon6 * epsilon6;
var bc = vec(4);
var ca = vec(4);
var ab = vec(4);
var at_b = vec(4);
var at_c = vec(4);
var bt_c = vec(4);
var bt_a = vec(4);
var ct_a = vec(4);
var ct_b = vec(4);
var bct = vec(8);
var cat = vec(8);
var abt = vec(8);
var u2 = vec(4);
var _8 = vec(8);
var _8b = vec(8);
var _16 = vec(16);
var _12 = vec(12);
var fin = vec(192);
var fin2 = vec(192);
// node_modules/robust-predicates/esm/incircle.js
var iccerrboundA = (10 + 96 * epsilon6) * epsilon6;
var iccerrboundB = (4 + 48 * epsilon6) * epsilon6;
var iccerrboundC = (44 + 576 * epsilon6) * epsilon6 * epsilon6;
var bc2 = vec(4);
var ca2 = vec(4);
var ab2 = vec(4);
var aa = vec(4);
var bb = vec(4);
var cc = vec(4);
var u3 = vec(4);
var v = vec(4);
var axtbc = vec(8);
var aytbc = vec(8);
var bxtca = vec(8);
var bytca = vec(8);
var cxtab = vec(8);
var cytab = vec(8);
var abt2 = vec(8);
var bct2 = vec(8);
var cat2 = vec(8);
var abtt = vec(4);
var bctt = vec(4);
var catt = vec(4);
var _82 = vec(8);
var _162 = vec(16);
var _16b = vec(16);
var _16c = vec(16);
var _32 = vec(32);
var _32b = vec(32);
var _48 = vec(48);
var _64 = vec(64);
var fin3 = vec(1152);
var fin22 = vec(1152);
// node_modules/robust-predicates/esm/insphere.js
var isperrboundA = (16 + 224 * epsilon6) * epsilon6;
var isperrboundB = (5 + 72 * epsilon6) * epsilon6;
var isperrboundC = (71 + 1408 * epsilon6) * epsilon6 * epsilon6;
var ab3 = vec(4);
var bc3 = vec(4);
var cd = vec(4);
var de = vec(4);
var ea = vec(4);
var ac = vec(4);
var bd = vec(4);
var ce = vec(4);
var da = vec(4);
var eb = vec(4);
var abc = vec(24);
var bcd = vec(24);
var cde = vec(24);
var dea = vec(24);
var eab = vec(24);
var abd = vec(24);
var bce = vec(24);
var cda = vec(24);
var deb = vec(24);
var eac = vec(24);
var adet = vec(1152);
var bdet = vec(1152);
var cdet = vec(1152);
var ddet = vec(1152);
var edet = vec(1152);
var abdet = vec(2304);
var cddet = vec(2304);
var cdedet = vec(3456);
var deter = vec(5760);
var _83 = vec(8);
var _8b2 = vec(8);
var _8c = vec(8);
var _163 = vec(16);
var _24 = vec(24);
var _482 = vec(48);
var _48b = vec(48);
var _96 = vec(96);
var _192 = vec(192);
var _384x = vec(384);
var _384y = vec(384);
var _384z = vec(384);
var _768 = vec(768);
var xdet = vec(96);
var ydet = vec(96);
var zdet = vec(96);
var fin5 = vec(1152);
// node_modules/delaunator/index.js
var EPSILON = Math.pow(2, -52);
var EDGE_STACK = new Uint32Array(512);

class Delaunator {
  static from(points, getX = defaultGetX, getY = defaultGetY) {
    const n = points.length;
    const coords = new Float64Array(n * 2);
    for (let i = 0;i < n; i++) {
      const p = points[i];
      coords[2 * i] = getX(p);
      coords[2 * i + 1] = getY(p);
    }
    return new Delaunator(coords);
  }
  constructor(coords) {
    const n = coords.length >> 1;
    if (n > 0 && typeof coords[0] !== "number")
      throw new Error("Expected coords to contain numbers.");
    this.coords = coords;
    const maxTriangles = Math.max(2 * n - 5, 0);
    this._triangles = new Uint32Array(maxTriangles * 3);
    this._halfedges = new Int32Array(maxTriangles * 3);
    this._hashSize = Math.ceil(Math.sqrt(n));
    this._hullPrev = new Uint32Array(n);
    this._hullNext = new Uint32Array(n);
    this._hullTri = new Uint32Array(n);
    this._hullHash = new Int32Array(this._hashSize);
    this._ids = new Uint32Array(n);
    this._dists = new Float64Array(n);
    this.trianglesLen = 0;
    this._cx = 0;
    this._cy = 0;
    this._hullStart = 0;
    this.hull = this._triangles;
    this.triangles = this._triangles;
    this.halfedges = this._halfedges;
    this.update();
  }
  update() {
    const { coords, _hullPrev: hullPrev, _hullNext: hullNext, _hullTri: hullTri, _hullHash: hullHash } = this;
    const n = coords.length >> 1;
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    for (let i = 0;i < n; i++) {
      const x = coords[2 * i];
      const y = coords[2 * i + 1];
      if (x < minX)
        minX = x;
      if (y < minY)
        minY = y;
      if (x > maxX)
        maxX = x;
      if (y > maxY)
        maxY = y;
      this._ids[i] = i;
    }
    const cx = (minX + maxX) / 2;
    const cy = (minY + maxY) / 2;
    let i0 = 0, i1 = 0, i2 = 0;
    for (let i = 0, minDist = Infinity;i < n; i++) {
      const d = dist(cx, cy, coords[2 * i], coords[2 * i + 1]);
      if (d < minDist) {
        i0 = i;
        minDist = d;
      }
    }
    const i0x = coords[2 * i0];
    const i0y = coords[2 * i0 + 1];
    for (let i = 0, minDist = Infinity;i < n; i++) {
      if (i === i0)
        continue;
      const d = dist(i0x, i0y, coords[2 * i], coords[2 * i + 1]);
      if (d < minDist && d > 0) {
        i1 = i;
        minDist = d;
      }
    }
    let i1x = coords[2 * i1];
    let i1y = coords[2 * i1 + 1];
    let minRadius = Infinity;
    for (let i = 0;i < n; i++) {
      if (i === i0 || i === i1)
        continue;
      const r = circumradius(i0x, i0y, i1x, i1y, coords[2 * i], coords[2 * i + 1]);
      if (r < minRadius) {
        i2 = i;
        minRadius = r;
      }
    }
    let i2x = coords[2 * i2];
    let i2y = coords[2 * i2 + 1];
    if (minRadius === Infinity) {
      for (let i = 0;i < n; i++) {
        this._dists[i] = coords[2 * i] - coords[0] || coords[2 * i + 1] - coords[1];
      }
      quicksort(this._ids, this._dists, 0, n - 1);
      const hull = new Uint32Array(n);
      let j = 0;
      for (let i = 0, d0 = -Infinity;i < n; i++) {
        const id2 = this._ids[i];
        const d = this._dists[id2];
        if (d > d0) {
          hull[j++] = id2;
          d0 = d;
        }
      }
      this.hull = hull.subarray(0, j);
      this.triangles = new Uint32Array(0);
      this.halfedges = new Int32Array(0);
      return;
    }
    if (orient2d(i0x, i0y, i1x, i1y, i2x, i2y) < 0) {
      const i = i1;
      const x = i1x;
      const y = i1y;
      i1 = i2;
      i1x = i2x;
      i1y = i2y;
      i2 = i;
      i2x = x;
      i2y = y;
    }
    const center2 = circumcenter(i0x, i0y, i1x, i1y, i2x, i2y);
    this._cx = center2.x;
    this._cy = center2.y;
    for (let i = 0;i < n; i++) {
      this._dists[i] = dist(coords[2 * i], coords[2 * i + 1], center2.x, center2.y);
    }
    quicksort(this._ids, this._dists, 0, n - 1);
    this._hullStart = i0;
    let hullSize = 3;
    hullNext[i0] = hullPrev[i2] = i1;
    hullNext[i1] = hullPrev[i0] = i2;
    hullNext[i2] = hullPrev[i1] = i0;
    hullTri[i0] = 0;
    hullTri[i1] = 1;
    hullTri[i2] = 2;
    hullHash.fill(-1);
    hullHash[this._hashKey(i0x, i0y)] = i0;
    hullHash[this._hashKey(i1x, i1y)] = i1;
    hullHash[this._hashKey(i2x, i2y)] = i2;
    this.trianglesLen = 0;
    this._addTriangle(i0, i1, i2, -1, -1, -1);
    for (let k = 0, xp = 0, yp = 0;k < this._ids.length; k++) {
      const i = this._ids[k];
      const x = coords[2 * i];
      const y = coords[2 * i + 1];
      if (k > 0 && Math.abs(x - xp) <= EPSILON && Math.abs(y - yp) <= EPSILON)
        continue;
      xp = x;
      yp = y;
      if (i === i0 || i === i1 || i === i2)
        continue;
      let start2 = 0;
      for (let j = 0, key = this._hashKey(x, y);j < this._hashSize; j++) {
        start2 = hullHash[(key + j) % this._hashSize];
        if (start2 !== -1 && start2 !== hullNext[start2])
          break;
      }
      start2 = hullPrev[start2];
      let e = start2, q;
      while (q = hullNext[e], orient2d(x, y, coords[2 * e], coords[2 * e + 1], coords[2 * q], coords[2 * q + 1]) >= 0) {
        e = q;
        if (e === start2) {
          e = -1;
          break;
        }
      }
      if (e === -1)
        continue;
      let t = this._addTriangle(e, i, hullNext[e], -1, -1, hullTri[e]);
      hullTri[i] = this._legalize(t + 2);
      hullTri[e] = t;
      hullSize++;
      let n2 = hullNext[e];
      while (q = hullNext[n2], orient2d(x, y, coords[2 * n2], coords[2 * n2 + 1], coords[2 * q], coords[2 * q + 1]) < 0) {
        t = this._addTriangle(n2, i, q, hullTri[i], -1, hullTri[n2]);
        hullTri[i] = this._legalize(t + 2);
        hullNext[n2] = n2;
        hullSize--;
        n2 = q;
      }
      if (e === start2) {
        while (q = hullPrev[e], orient2d(x, y, coords[2 * q], coords[2 * q + 1], coords[2 * e], coords[2 * e + 1]) < 0) {
          t = this._addTriangle(q, i, e, -1, hullTri[e], hullTri[q]);
          this._legalize(t + 2);
          hullTri[q] = t;
          hullNext[e] = e;
          hullSize--;
          e = q;
        }
      }
      this._hullStart = hullPrev[i] = e;
      hullNext[e] = hullPrev[n2] = i;
      hullNext[i] = n2;
      hullHash[this._hashKey(x, y)] = i;
      hullHash[this._hashKey(coords[2 * e], coords[2 * e + 1])] = e;
    }
    this.hull = new Uint32Array(hullSize);
    for (let i = 0, e = this._hullStart;i < hullSize; i++) {
      this.hull[i] = e;
      e = hullNext[e];
    }
    this.triangles = this._triangles.subarray(0, this.trianglesLen);
    this.halfedges = this._halfedges.subarray(0, this.trianglesLen);
  }
  _hashKey(x, y) {
    return Math.floor(pseudoAngle(x - this._cx, y - this._cy) * this._hashSize) % this._hashSize;
  }
  _legalize(a) {
    const { _triangles: triangles, _halfedges: halfedges, coords } = this;
    let i = 0;
    let ar = 0;
    while (true) {
      const b = halfedges[a];
      const a0 = a - a % 3;
      ar = a0 + (a + 2) % 3;
      if (b === -1) {
        if (i === 0)
          break;
        a = EDGE_STACK[--i];
        continue;
      }
      const b02 = b - b % 3;
      const al = a0 + (a + 1) % 3;
      const bl = b02 + (b + 2) % 3;
      const p0 = triangles[ar];
      const pr = triangles[a];
      const pl = triangles[al];
      const p1 = triangles[bl];
      const illegal = inCircle(coords[2 * p0], coords[2 * p0 + 1], coords[2 * pr], coords[2 * pr + 1], coords[2 * pl], coords[2 * pl + 1], coords[2 * p1], coords[2 * p1 + 1]);
      if (illegal) {
        triangles[a] = p1;
        triangles[b] = p0;
        const hbl = halfedges[bl];
        if (hbl === -1) {
          let e = this._hullStart;
          do {
            if (this._hullTri[e] === bl) {
              this._hullTri[e] = a;
              break;
            }
            e = this._hullPrev[e];
          } while (e !== this._hullStart);
        }
        this._link(a, hbl);
        this._link(b, halfedges[ar]);
        this._link(ar, bl);
        const br = b02 + (b + 1) % 3;
        if (i < EDGE_STACK.length) {
          EDGE_STACK[i++] = br;
        }
      } else {
        if (i === 0)
          break;
        a = EDGE_STACK[--i];
      }
    }
    return ar;
  }
  _link(a, b) {
    this._halfedges[a] = b;
    if (b !== -1)
      this._halfedges[b] = a;
  }
  _addTriangle(i0, i1, i2, a, b, c) {
    const t = this.trianglesLen;
    this._triangles[t] = i0;
    this._triangles[t + 1] = i1;
    this._triangles[t + 2] = i2;
    this._link(t, a);
    this._link(t + 1, b);
    this._link(t + 2, c);
    this.trianglesLen += 3;
    return t;
  }
}
function pseudoAngle(dx, dy) {
  const p = dx / (Math.abs(dx) + Math.abs(dy));
  return (dy > 0 ? 3 - p : 1 + p) / 4;
}
function dist(ax, ay, bx, by) {
  const dx = ax - bx;
  const dy = ay - by;
  return dx * dx + dy * dy;
}
function inCircle(ax, ay, bx, by, cx, cy, px, py) {
  const dx = ax - px;
  const dy = ay - py;
  const ex = bx - px;
  const ey = by - py;
  const fx = cx - px;
  const fy = cy - py;
  const ap = dx * dx + dy * dy;
  const bp = ex * ex + ey * ey;
  const cp = fx * fx + fy * fy;
  return dx * (ey * cp - bp * fy) - dy * (ex * cp - bp * fx) + ap * (ex * fy - ey * fx) < 0;
}
function circumradius(ax, ay, bx, by, cx, cy) {
  const dx = bx - ax;
  const dy = by - ay;
  const ex = cx - ax;
  const ey = cy - ay;
  const bl = dx * dx + dy * dy;
  const cl = ex * ex + ey * ey;
  const d = 0.5 / (dx * ey - dy * ex);
  const x = (ey * bl - dy * cl) * d;
  const y = (dx * cl - ex * bl) * d;
  return x * x + y * y;
}
function circumcenter(ax, ay, bx, by, cx, cy) {
  const dx = bx - ax;
  const dy = by - ay;
  const ex = cx - ax;
  const ey = cy - ay;
  const bl = dx * dx + dy * dy;
  const cl = ex * ex + ey * ey;
  const d = 0.5 / (dx * ey - dy * ex);
  const x = ax + (ey * bl - dy * cl) * d;
  const y = ay + (dx * cl - ex * bl) * d;
  return { x, y };
}
function quicksort(ids, dists, left2, right2) {
  if (right2 - left2 <= 20) {
    for (let i = left2 + 1;i <= right2; i++) {
      const temp = ids[i];
      const tempDist = dists[temp];
      let j = i - 1;
      while (j >= left2 && dists[ids[j]] > tempDist)
        ids[j + 1] = ids[j--];
      ids[j + 1] = temp;
    }
  } else {
    const median2 = left2 + right2 >> 1;
    let i = left2 + 1;
    let j = right2;
    swap2(ids, median2, i);
    if (dists[ids[left2]] > dists[ids[right2]])
      swap2(ids, left2, right2);
    if (dists[ids[i]] > dists[ids[right2]])
      swap2(ids, i, right2);
    if (dists[ids[left2]] > dists[ids[i]])
      swap2(ids, left2, i);
    const temp = ids[i];
    const tempDist = dists[temp];
    while (true) {
      do
        i++;
      while (dists[ids[i]] < tempDist);
      do
        j--;
      while (dists[ids[j]] > tempDist);
      if (j < i)
        break;
      swap2(ids, i, j);
    }
    ids[left2 + 1] = ids[j];
    ids[j] = temp;
    if (right2 - i + 1 >= j - left2) {
      quicksort(ids, dists, i, right2);
      quicksort(ids, dists, left2, j - 1);
    } else {
      quicksort(ids, dists, left2, j - 1);
      quicksort(ids, dists, i, right2);
    }
  }
}
function swap2(arr, i, j) {
  const tmp = arr[i];
  arr[i] = arr[j];
  arr[j] = tmp;
}
function defaultGetX(p) {
  return p[0];
}
function defaultGetY(p) {
  return p[1];
}

// node_modules/d3-delaunay/src/path.js
var epsilon7 = 0.000001;

class Path2 {
  constructor() {
    this._x0 = this._y0 = this._x1 = this._y1 = null;
    this._ = "";
  }
  moveTo(x, y) {
    this._ += `M${this._x0 = this._x1 = +x},${this._y0 = this._y1 = +y}`;
  }
  closePath() {
    if (this._x1 !== null) {
      this._x1 = this._x0, this._y1 = this._y0;
      this._ += "Z";
    }
  }
  lineTo(x, y) {
    this._ += `L${this._x1 = +x},${this._y1 = +y}`;
  }
  arc(x, y, r) {
    x = +x, y = +y, r = +r;
    const x0 = x + r;
    const y0 = y;
    if (r < 0)
      throw new Error("negative radius");
    if (this._x1 === null)
      this._ += `M${x0},${y0}`;
    else if (Math.abs(this._x1 - x0) > epsilon7 || Math.abs(this._y1 - y0) > epsilon7)
      this._ += "L" + x0 + "," + y0;
    if (!r)
      return;
    this._ += `A${r},${r},0,1,1,${x - r},${y}A${r},${r},0,1,1,${this._x1 = x0},${this._y1 = y0}`;
  }
  rect(x, y, w, h) {
    this._ += `M${this._x0 = this._x1 = +x},${this._y0 = this._y1 = +y}h${+w}v${+h}h${-w}Z`;
  }
  value() {
    return this._ || null;
  }
}

// node_modules/d3-delaunay/src/polygon.js
class Polygon {
  constructor() {
    this._ = [];
  }
  moveTo(x, y) {
    this._.push([x, y]);
  }
  closePath() {
    this._.push(this._[0].slice());
  }
  lineTo(x, y) {
    this._.push([x, y]);
  }
  value() {
    return this._.length ? this._ : null;
  }
}

// node_modules/d3-delaunay/src/voronoi.js
class Voronoi {
  constructor(delaunay, [xmin, ymin, xmax, ymax] = [0, 0, 960, 500]) {
    if (!((xmax = +xmax) >= (xmin = +xmin)) || !((ymax = +ymax) >= (ymin = +ymin)))
      throw new Error("invalid bounds");
    this.delaunay = delaunay;
    this._circumcenters = new Float64Array(delaunay.points.length * 2);
    this.vectors = new Float64Array(delaunay.points.length * 2);
    this.xmax = xmax, this.xmin = xmin;
    this.ymax = ymax, this.ymin = ymin;
    this._init();
  }
  update() {
    this.delaunay.update();
    this._init();
    return this;
  }
  _init() {
    const { delaunay: { points, hull, triangles }, vectors } = this;
    let bx, by;
    const circumcenters = this.circumcenters = this._circumcenters.subarray(0, triangles.length / 3 * 2);
    for (let i = 0, j = 0, n = triangles.length, x, y;i < n; i += 3, j += 2) {
      const t12 = triangles[i] * 2;
      const t22 = triangles[i + 1] * 2;
      const t32 = triangles[i + 2] * 2;
      const x12 = points[t12];
      const y12 = points[t12 + 1];
      const x2 = points[t22];
      const y2 = points[t22 + 1];
      const x3 = points[t32];
      const y3 = points[t32 + 1];
      const dx = x2 - x12;
      const dy = y2 - y12;
      const ex = x3 - x12;
      const ey = y3 - y12;
      const ab4 = (dx * ey - dy * ex) * 2;
      if (Math.abs(ab4) < 0.000000001) {
        if (bx === undefined) {
          bx = by = 0;
          for (const i2 of hull)
            bx += points[i2 * 2], by += points[i2 * 2 + 1];
          bx /= hull.length, by /= hull.length;
        }
        const a = 1e9 * Math.sign((bx - x12) * ey - (by - y12) * ex);
        x = (x12 + x3) / 2 - a * ey;
        y = (y12 + y3) / 2 + a * ex;
      } else {
        const d = 1 / ab4;
        const bl = dx * dx + dy * dy;
        const cl = ex * ex + ey * ey;
        x = x12 + (ey * bl - dy * cl) * d;
        y = y12 + (dx * cl - ex * bl) * d;
      }
      circumcenters[j] = x;
      circumcenters[j + 1] = y;
    }
    let h = hull[hull.length - 1];
    let p0, p1 = h * 4;
    let x0, x1 = points[2 * h];
    let y0, y1 = points[2 * h + 1];
    vectors.fill(0);
    for (let i = 0;i < hull.length; ++i) {
      h = hull[i];
      p0 = p1, x0 = x1, y0 = y1;
      p1 = h * 4, x1 = points[2 * h], y1 = points[2 * h + 1];
      vectors[p0 + 2] = vectors[p1] = y0 - y1;
      vectors[p0 + 3] = vectors[p1 + 1] = x1 - x0;
    }
  }
  render(context) {
    const buffer = context == null ? context = new Path2 : undefined;
    const { delaunay: { halfedges, inedges, hull }, circumcenters, vectors } = this;
    if (hull.length <= 1)
      return null;
    for (let i = 0, n = halfedges.length;i < n; ++i) {
      const j = halfedges[i];
      if (j < i)
        continue;
      const ti = Math.floor(i / 3) * 2;
      const tj = Math.floor(j / 3) * 2;
      const xi = circumcenters[ti];
      const yi = circumcenters[ti + 1];
      const xj = circumcenters[tj];
      const yj = circumcenters[tj + 1];
      this._renderSegment(xi, yi, xj, yj, context);
    }
    let h0, h1 = hull[hull.length - 1];
    for (let i = 0;i < hull.length; ++i) {
      h0 = h1, h1 = hull[i];
      const t = Math.floor(inedges[h1] / 3) * 2;
      const x = circumcenters[t];
      const y = circumcenters[t + 1];
      const v2 = h0 * 4;
      const p = this._project(x, y, vectors[v2 + 2], vectors[v2 + 3]);
      if (p)
        this._renderSegment(x, y, p[0], p[1], context);
    }
    return buffer && buffer.value();
  }
  renderBounds(context) {
    const buffer = context == null ? context = new Path2 : undefined;
    context.rect(this.xmin, this.ymin, this.xmax - this.xmin, this.ymax - this.ymin);
    return buffer && buffer.value();
  }
  renderCell(i, context) {
    const buffer = context == null ? context = new Path2 : undefined;
    const points = this._clip(i);
    if (points === null || !points.length)
      return;
    context.moveTo(points[0], points[1]);
    let n = points.length;
    while (points[0] === points[n - 2] && points[1] === points[n - 1] && n > 1)
      n -= 2;
    for (let i2 = 2;i2 < n; i2 += 2) {
      if (points[i2] !== points[i2 - 2] || points[i2 + 1] !== points[i2 - 1])
        context.lineTo(points[i2], points[i2 + 1]);
    }
    context.closePath();
    return buffer && buffer.value();
  }
  *cellPolygons() {
    const { delaunay: { points } } = this;
    for (let i = 0, n = points.length / 2;i < n; ++i) {
      const cell = this.cellPolygon(i);
      if (cell)
        cell.index = i, yield cell;
    }
  }
  cellPolygon(i) {
    const polygon = new Polygon;
    this.renderCell(i, polygon);
    return polygon.value();
  }
  _renderSegment(x0, y0, x1, y1, context) {
    let S;
    const c0 = this._regioncode(x0, y0);
    const c1 = this._regioncode(x1, y1);
    if (c0 === 0 && c1 === 0) {
      context.moveTo(x0, y0);
      context.lineTo(x1, y1);
    } else if (S = this._clipSegment(x0, y0, x1, y1, c0, c1)) {
      context.moveTo(S[0], S[1]);
      context.lineTo(S[2], S[3]);
    }
  }
  contains(i, x, y) {
    if ((x = +x, x !== x) || (y = +y, y !== y))
      return false;
    return this.delaunay._step(i, x, y) === i;
  }
  *neighbors(i) {
    const ci = this._clip(i);
    if (ci)
      for (const j of this.delaunay.neighbors(i)) {
        const cj = this._clip(j);
        if (cj)
          loop:
            for (let ai = 0, li = ci.length;ai < li; ai += 2) {
              for (let aj = 0, lj = cj.length;aj < lj; aj += 2) {
                if (ci[ai] === cj[aj] && ci[ai + 1] === cj[aj + 1] && ci[(ai + 2) % li] === cj[(aj + lj - 2) % lj] && ci[(ai + 3) % li] === cj[(aj + lj - 1) % lj]) {
                  yield j;
                  break loop;
                }
              }
            }
      }
  }
  _cell(i) {
    const { circumcenters, delaunay: { inedges, halfedges, triangles } } = this;
    const e0 = inedges[i];
    if (e0 === -1)
      return null;
    const points = [];
    let e = e0;
    do {
      const t = Math.floor(e / 3);
      points.push(circumcenters[t * 2], circumcenters[t * 2 + 1]);
      e = e % 3 === 2 ? e - 2 : e + 1;
      if (triangles[e] !== i)
        break;
      e = halfedges[e];
    } while (e !== e0 && e !== -1);
    return points;
  }
  _clip(i) {
    if (i === 0 && this.delaunay.hull.length === 1) {
      return [this.xmax, this.ymin, this.xmax, this.ymax, this.xmin, this.ymax, this.xmin, this.ymin];
    }
    const points = this._cell(i);
    if (points === null)
      return null;
    const { vectors: V } = this;
    const v2 = i * 4;
    return this._simplify(V[v2] || V[v2 + 1] ? this._clipInfinite(i, points, V[v2], V[v2 + 1], V[v2 + 2], V[v2 + 3]) : this._clipFinite(i, points));
  }
  _clipFinite(i, points) {
    const n = points.length;
    let P = null;
    let x0, y0, x1 = points[n - 2], y1 = points[n - 1];
    let c0, c1 = this._regioncode(x1, y1);
    let e0, e1 = 0;
    for (let j = 0;j < n; j += 2) {
      x0 = x1, y0 = y1, x1 = points[j], y1 = points[j + 1];
      c0 = c1, c1 = this._regioncode(x1, y1);
      if (c0 === 0 && c1 === 0) {
        e0 = e1, e1 = 0;
        if (P)
          P.push(x1, y1);
        else
          P = [x1, y1];
      } else {
        let S, sx0, sy0, sx1, sy1;
        if (c0 === 0) {
          if ((S = this._clipSegment(x0, y0, x1, y1, c0, c1)) === null)
            continue;
          [sx0, sy0, sx1, sy1] = S;
        } else {
          if ((S = this._clipSegment(x1, y1, x0, y0, c1, c0)) === null)
            continue;
          [sx1, sy1, sx0, sy0] = S;
          e0 = e1, e1 = this._edgecode(sx0, sy0);
          if (e0 && e1)
            this._edge(i, e0, e1, P, P.length);
          if (P)
            P.push(sx0, sy0);
          else
            P = [sx0, sy0];
        }
        e0 = e1, e1 = this._edgecode(sx1, sy1);
        if (e0 && e1)
          this._edge(i, e0, e1, P, P.length);
        if (P)
          P.push(sx1, sy1);
        else
          P = [sx1, sy1];
      }
    }
    if (P) {
      e0 = e1, e1 = this._edgecode(P[0], P[1]);
      if (e0 && e1)
        this._edge(i, e0, e1, P, P.length);
    } else if (this.contains(i, (this.xmin + this.xmax) / 2, (this.ymin + this.ymax) / 2)) {
      return [this.xmax, this.ymin, this.xmax, this.ymax, this.xmin, this.ymax, this.xmin, this.ymin];
    }
    return P;
  }
  _clipSegment(x0, y0, x1, y1, c0, c1) {
    const flip = c0 < c1;
    if (flip)
      [x0, y0, x1, y1, c0, c1] = [x1, y1, x0, y0, c1, c0];
    while (true) {
      if (c0 === 0 && c1 === 0)
        return flip ? [x1, y1, x0, y0] : [x0, y0, x1, y1];
      if (c0 & c1)
        return null;
      let x, y, c = c0 || c1;
      if (c & 8)
        x = x0 + (x1 - x0) * (this.ymax - y0) / (y1 - y0), y = this.ymax;
      else if (c & 4)
        x = x0 + (x1 - x0) * (this.ymin - y0) / (y1 - y0), y = this.ymin;
      else if (c & 2)
        y = y0 + (y1 - y0) * (this.xmax - x0) / (x1 - x0), x = this.xmax;
      else
        y = y0 + (y1 - y0) * (this.xmin - x0) / (x1 - x0), x = this.xmin;
      if (c0)
        x0 = x, y0 = y, c0 = this._regioncode(x0, y0);
      else
        x1 = x, y1 = y, c1 = this._regioncode(x1, y1);
    }
  }
  _clipInfinite(i, points, vx0, vy0, vxn, vyn) {
    let P = Array.from(points), p;
    if (p = this._project(P[0], P[1], vx0, vy0))
      P.unshift(p[0], p[1]);
    if (p = this._project(P[P.length - 2], P[P.length - 1], vxn, vyn))
      P.push(p[0], p[1]);
    if (P = this._clipFinite(i, P)) {
      for (let j = 0, n = P.length, c0, c1 = this._edgecode(P[n - 2], P[n - 1]);j < n; j += 2) {
        c0 = c1, c1 = this._edgecode(P[j], P[j + 1]);
        if (c0 && c1)
          j = this._edge(i, c0, c1, P, j), n = P.length;
      }
    } else if (this.contains(i, (this.xmin + this.xmax) / 2, (this.ymin + this.ymax) / 2)) {
      P = [this.xmin, this.ymin, this.xmax, this.ymin, this.xmax, this.ymax, this.xmin, this.ymax];
    }
    return P;
  }
  _edge(i, e0, e1, P, j) {
    while (e0 !== e1) {
      let x, y;
      switch (e0) {
        case 5:
          e0 = 4;
          continue;
        case 4:
          e0 = 6, x = this.xmax, y = this.ymin;
          break;
        case 6:
          e0 = 2;
          continue;
        case 2:
          e0 = 10, x = this.xmax, y = this.ymax;
          break;
        case 10:
          e0 = 8;
          continue;
        case 8:
          e0 = 9, x = this.xmin, y = this.ymax;
          break;
        case 9:
          e0 = 1;
          continue;
        case 1:
          e0 = 5, x = this.xmin, y = this.ymin;
          break;
      }
      if ((P[j] !== x || P[j + 1] !== y) && this.contains(i, x, y)) {
        P.splice(j, 0, x, y), j += 2;
      }
    }
    return j;
  }
  _project(x0, y0, vx, vy) {
    let t = Infinity, c, x, y;
    if (vy < 0) {
      if (y0 <= this.ymin)
        return null;
      if ((c = (this.ymin - y0) / vy) < t)
        y = this.ymin, x = x0 + (t = c) * vx;
    } else if (vy > 0) {
      if (y0 >= this.ymax)
        return null;
      if ((c = (this.ymax - y0) / vy) < t)
        y = this.ymax, x = x0 + (t = c) * vx;
    }
    if (vx > 0) {
      if (x0 >= this.xmax)
        return null;
      if ((c = (this.xmax - x0) / vx) < t)
        x = this.xmax, y = y0 + (t = c) * vy;
    } else if (vx < 0) {
      if (x0 <= this.xmin)
        return null;
      if ((c = (this.xmin - x0) / vx) < t)
        x = this.xmin, y = y0 + (t = c) * vy;
    }
    return [x, y];
  }
  _edgecode(x, y) {
    return (x === this.xmin ? 1 : x === this.xmax ? 2 : 0) | (y === this.ymin ? 4 : y === this.ymax ? 8 : 0);
  }
  _regioncode(x, y) {
    return (x < this.xmin ? 1 : x > this.xmax ? 2 : 0) | (y < this.ymin ? 4 : y > this.ymax ? 8 : 0);
  }
  _simplify(P) {
    if (P && P.length > 4) {
      for (let i = 0;i < P.length; i += 2) {
        const j = (i + 2) % P.length, k = (i + 4) % P.length;
        if (P[i] === P[j] && P[j] === P[k] || P[i + 1] === P[j + 1] && P[j + 1] === P[k + 1]) {
          P.splice(j, 2), i -= 2;
        }
      }
      if (!P.length)
        P = null;
    }
    return P;
  }
}

// node_modules/d3-delaunay/src/delaunay.js
var tau4 = 2 * Math.PI;
var pow = Math.pow;
function pointX(p) {
  return p[0];
}
function pointY(p) {
  return p[1];
}
function collinear2(d) {
  const { triangles, coords } = d;
  for (let i = 0;i < triangles.length; i += 3) {
    const a = 2 * triangles[i], b = 2 * triangles[i + 1], c = 2 * triangles[i + 2], cross2 = (coords[c] - coords[a]) * (coords[b + 1] - coords[a + 1]) - (coords[b] - coords[a]) * (coords[c + 1] - coords[a + 1]);
    if (cross2 > 0.0000000001)
      return false;
  }
  return true;
}
function jitter(x, y, r) {
  return [x + Math.sin(x + y) * r, y + Math.cos(x - y) * r];
}

class Delaunay {
  static from(points, fx = pointX, fy = pointY, that) {
    return new Delaunay("length" in points ? flatArray(points, fx, fy, that) : Float64Array.from(flatIterable(points, fx, fy, that)));
  }
  constructor(points) {
    this._delaunator = new Delaunator(points);
    this.inedges = new Int32Array(points.length / 2);
    this._hullIndex = new Int32Array(points.length / 2);
    this.points = this._delaunator.coords;
    this._init();
  }
  update() {
    this._delaunator.update();
    this._init();
    return this;
  }
  _init() {
    const d = this._delaunator, points = this.points;
    if (d.hull && d.hull.length > 2 && collinear2(d)) {
      this.collinear = Int32Array.from({ length: points.length / 2 }, (_, i) => i).sort((i, j) => points[2 * i] - points[2 * j] || points[2 * i + 1] - points[2 * j + 1]);
      const e = this.collinear[0], f = this.collinear[this.collinear.length - 1], bounds = [points[2 * e], points[2 * e + 1], points[2 * f], points[2 * f + 1]], r = 0.00000001 * Math.hypot(bounds[3] - bounds[1], bounds[2] - bounds[0]);
      for (let i = 0, n = points.length / 2;i < n; ++i) {
        const p = jitter(points[2 * i], points[2 * i + 1], r);
        points[2 * i] = p[0];
        points[2 * i + 1] = p[1];
      }
      this._delaunator = new Delaunator(points);
    } else {
      delete this.collinear;
    }
    const halfedges = this.halfedges = this._delaunator.halfedges;
    const hull = this.hull = this._delaunator.hull;
    const triangles = this.triangles = this._delaunator.triangles;
    const inedges = this.inedges.fill(-1);
    const hullIndex = this._hullIndex.fill(-1);
    for (let e = 0, n = halfedges.length;e < n; ++e) {
      const p = triangles[e % 3 === 2 ? e - 2 : e + 1];
      if (halfedges[e] === -1 || inedges[p] === -1)
        inedges[p] = e;
    }
    for (let i = 0, n = hull.length;i < n; ++i) {
      hullIndex[hull[i]] = i;
    }
    if (hull.length <= 2 && hull.length > 0) {
      this.triangles = new Int32Array(3).fill(-1);
      this.halfedges = new Int32Array(3).fill(-1);
      this.triangles[0] = hull[0];
      inedges[hull[0]] = 1;
      if (hull.length === 2) {
        inedges[hull[1]] = 0;
        this.triangles[1] = hull[1];
        this.triangles[2] = hull[1];
      }
    }
  }
  voronoi(bounds) {
    return new Voronoi(this, bounds);
  }
  *neighbors(i) {
    const { inedges, hull, _hullIndex, halfedges, triangles, collinear: collinear3 } = this;
    if (collinear3) {
      const l = collinear3.indexOf(i);
      if (l > 0)
        yield collinear3[l - 1];
      if (l < collinear3.length - 1)
        yield collinear3[l + 1];
      return;
    }
    const e0 = inedges[i];
    if (e0 === -1)
      return;
    let e = e0, p0 = -1;
    do {
      yield p0 = triangles[e];
      e = e % 3 === 2 ? e - 2 : e + 1;
      if (triangles[e] !== i)
        return;
      e = halfedges[e];
      if (e === -1) {
        const p = hull[(_hullIndex[i] + 1) % hull.length];
        if (p !== p0)
          yield p;
        return;
      }
    } while (e !== e0);
  }
  find(x, y, i = 0) {
    if ((x = +x, x !== x) || (y = +y, y !== y))
      return -1;
    const i0 = i;
    let c;
    while ((c = this._step(i, x, y)) >= 0 && c !== i && c !== i0)
      i = c;
    return c;
  }
  _step(i, x, y) {
    const { inedges, hull, _hullIndex, halfedges, triangles, points } = this;
    if (inedges[i] === -1 || !points.length)
      return (i + 1) % (points.length >> 1);
    let c = i;
    let dc = pow(x - points[i * 2], 2) + pow(y - points[i * 2 + 1], 2);
    const e0 = inedges[i];
    let e = e0;
    do {
      let t = triangles[e];
      const dt = pow(x - points[t * 2], 2) + pow(y - points[t * 2 + 1], 2);
      if (dt < dc)
        dc = dt, c = t;
      e = e % 3 === 2 ? e - 2 : e + 1;
      if (triangles[e] !== i)
        break;
      e = halfedges[e];
      if (e === -1) {
        e = hull[(_hullIndex[i] + 1) % hull.length];
        if (e !== t) {
          if (pow(x - points[e * 2], 2) + pow(y - points[e * 2 + 1], 2) < dc)
            return e;
        }
        break;
      }
    } while (e !== e0);
    return c;
  }
  render(context) {
    const buffer = context == null ? context = new Path2 : undefined;
    const { points, halfedges, triangles } = this;
    for (let i = 0, n = halfedges.length;i < n; ++i) {
      const j = halfedges[i];
      if (j < i)
        continue;
      const ti = triangles[i] * 2;
      const tj = triangles[j] * 2;
      context.moveTo(points[ti], points[ti + 1]);
      context.lineTo(points[tj], points[tj + 1]);
    }
    this.renderHull(context);
    return buffer && buffer.value();
  }
  renderPoints(context, r) {
    if (r === undefined && (!context || typeof context.moveTo !== "function"))
      r = context, context = null;
    r = r == undefined ? 2 : +r;
    const buffer = context == null ? context = new Path2 : undefined;
    const { points } = this;
    for (let i = 0, n = points.length;i < n; i += 2) {
      const x = points[i], y = points[i + 1];
      context.moveTo(x + r, y);
      context.arc(x, y, r, 0, tau4);
    }
    return buffer && buffer.value();
  }
  renderHull(context) {
    const buffer = context == null ? context = new Path2 : undefined;
    const { hull, points } = this;
    const h = hull[0] * 2, n = hull.length;
    context.moveTo(points[h], points[h + 1]);
    for (let i = 1;i < n; ++i) {
      const h2 = 2 * hull[i];
      context.lineTo(points[h2], points[h2 + 1]);
    }
    context.closePath();
    return buffer && buffer.value();
  }
  hullPolygon() {
    const polygon = new Polygon;
    this.renderHull(polygon);
    return polygon.value();
  }
  renderTriangle(i, context) {
    const buffer = context == null ? context = new Path2 : undefined;
    const { points, triangles } = this;
    const t02 = triangles[i *= 3] * 2;
    const t12 = triangles[i + 1] * 2;
    const t22 = triangles[i + 2] * 2;
    context.moveTo(points[t02], points[t02 + 1]);
    context.lineTo(points[t12], points[t12 + 1]);
    context.lineTo(points[t22], points[t22 + 1]);
    context.closePath();
    return buffer && buffer.value();
  }
  *trianglePolygons() {
    const { triangles } = this;
    for (let i = 0, n = triangles.length / 3;i < n; ++i) {
      yield this.trianglePolygon(i);
    }
  }
  trianglePolygon(i) {
    const polygon = new Polygon;
    this.renderTriangle(i, polygon);
    return polygon.value();
  }
}
function flatArray(points, fx, fy, that) {
  const n = points.length;
  const array4 = new Float64Array(n * 2);
  for (let i = 0;i < n; ++i) {
    const p = points[i];
    array4[i * 2] = fx.call(that, p, i, points);
    array4[i * 2 + 1] = fy.call(that, p, i, points);
  }
  return array4;
}
function* flatIterable(points, fx, fy, that) {
  let i = 0;
  for (const p of points) {
    yield fx.call(that, p, i, points);
    yield fy.call(that, p, i, points);
    ++i;
  }
}
// node_modules/d3-dsv/src/dsv.js
var EOL = {};
var EOF = {};
var QUOTE = 34;
var NEWLINE = 10;
var RETURN = 13;
function objectConverter(columns) {
  return new Function("d", "return {" + columns.map(function(name, i) {
    return JSON.stringify(name) + ": d[" + i + '] || ""';
  }).join(",") + "}");
}
function customConverter(columns, f) {
  var object = objectConverter(columns);
  return function(row, i) {
    return f(object(row), i, columns);
  };
}
function inferColumns(rows) {
  var columnSet = Object.create(null), columns = [];
  rows.forEach(function(row) {
    for (var column in row) {
      if (!(column in columnSet)) {
        columns.push(columnSet[column] = column);
      }
    }
  });
  return columns;
}
function pad(value, width) {
  var s = value + "", length3 = s.length;
  return length3 < width ? new Array(width - length3 + 1).join(0) + s : s;
}
function formatYear(year) {
  return year < 0 ? "-" + pad(-year, 6) : year > 9999 ? "+" + pad(year, 6) : pad(year, 4);
}
function formatDate(date) {
  var hours = date.getUTCHours(), minutes = date.getUTCMinutes(), seconds = date.getUTCSeconds(), milliseconds = date.getUTCMilliseconds();
  return isNaN(date) ? "Invalid Date" : formatYear(date.getUTCFullYear(), 4) + "-" + pad(date.getUTCMonth() + 1, 2) + "-" + pad(date.getUTCDate(), 2) + (milliseconds ? "T" + pad(hours, 2) + ":" + pad(minutes, 2) + ":" + pad(seconds, 2) + "." + pad(milliseconds, 3) + "Z" : seconds ? "T" + pad(hours, 2) + ":" + pad(minutes, 2) + ":" + pad(seconds, 2) + "Z" : minutes || hours ? "T" + pad(hours, 2) + ":" + pad(minutes, 2) + "Z" : "");
}
function dsv_default(delimiter) {
  var reFormat = new RegExp('["' + delimiter + `
\r]`), DELIMITER = delimiter.charCodeAt(0);
  function parse(text, f) {
    var convert, columns, rows = parseRows(text, function(row, i) {
      if (convert)
        return convert(row, i - 1);
      columns = row, convert = f ? customConverter(row, f) : objectConverter(row);
    });
    rows.columns = columns || [];
    return rows;
  }
  function parseRows(text, f) {
    var rows = [], N = text.length, I = 0, n = 0, t, eof = N <= 0, eol = false;
    if (text.charCodeAt(N - 1) === NEWLINE)
      --N;
    if (text.charCodeAt(N - 1) === RETURN)
      --N;
    function token() {
      if (eof)
        return EOF;
      if (eol)
        return eol = false, EOL;
      var i, j = I, c;
      if (text.charCodeAt(j) === QUOTE) {
        while (I++ < N && text.charCodeAt(I) !== QUOTE || text.charCodeAt(++I) === QUOTE)
          ;
        if ((i = I) >= N)
          eof = true;
        else if ((c = text.charCodeAt(I++)) === NEWLINE)
          eol = true;
        else if (c === RETURN) {
          eol = true;
          if (text.charCodeAt(I) === NEWLINE)
            ++I;
        }
        return text.slice(j + 1, i - 1).replace(/""/g, '"');
      }
      while (I < N) {
        if ((c = text.charCodeAt(i = I++)) === NEWLINE)
          eol = true;
        else if (c === RETURN) {
          eol = true;
          if (text.charCodeAt(I) === NEWLINE)
            ++I;
        } else if (c !== DELIMITER)
          continue;
        return text.slice(j, i);
      }
      return eof = true, text.slice(j, N);
    }
    while ((t = token()) !== EOF) {
      var row = [];
      while (t !== EOL && t !== EOF)
        row.push(t), t = token();
      if (f && (row = f(row, n++)) == null)
        continue;
      rows.push(row);
    }
    return rows;
  }
  function preformatBody(rows, columns) {
    return rows.map(function(row) {
      return columns.map(function(column) {
        return formatValue(row[column]);
      }).join(delimiter);
    });
  }
  function format(rows, columns) {
    if (columns == null)
      columns = inferColumns(rows);
    return [columns.map(formatValue).join(delimiter)].concat(preformatBody(rows, columns)).join(`
`);
  }
  function formatBody(rows, columns) {
    if (columns == null)
      columns = inferColumns(rows);
    return preformatBody(rows, columns).join(`
`);
  }
  function formatRows(rows) {
    return rows.map(formatRow).join(`
`);
  }
  function formatRow(row) {
    return row.map(formatValue).join(delimiter);
  }
  function formatValue(value) {
    return value == null ? "" : value instanceof Date ? formatDate(value) : reFormat.test(value += "") ? '"' + value.replace(/"/g, '""') + '"' : value;
  }
  return {
    parse,
    parseRows,
    format,
    formatBody,
    formatRows,
    formatRow,
    formatValue
  };
}
// node_modules/d3-dsv/src/csv.js
var csv = dsv_default(",");
var csvParse = csv.parse;
var csvParseRows = csv.parseRows;
var csvFormat = csv.format;
var csvFormatBody = csv.formatBody;
var csvFormatRows = csv.formatRows;
var csvFormatRow = csv.formatRow;
var csvFormatValue = csv.formatValue;
// node_modules/d3-dsv/src/tsv.js
var tsv = dsv_default("\t");
var tsvParse = tsv.parse;
var tsvParseRows = tsv.parseRows;
var tsvFormat = tsv.format;
var tsvFormatBody = tsv.formatBody;
var tsvFormatRows = tsv.formatRows;
var tsvFormatRow = tsv.formatRow;
var tsvFormatValue = tsv.formatValue;
// node_modules/d3-dsv/src/autoType.js
function autoType(object) {
  for (var key in object) {
    var value = object[key].trim(), number3, m;
    if (!value)
      value = null;
    else if (value === "true")
      value = true;
    else if (value === "false")
      value = false;
    else if (value === "NaN")
      value = NaN;
    else if (!isNaN(number3 = +value))
      value = number3;
    else if (m = value.match(/^([-+]\d{2})?\d{4}(-\d{2}(-\d{2})?)?(T\d{2}:\d{2}(:\d{2}(\.\d{3})?)?(Z|[-+]\d{2}:\d{2})?)?$/)) {
      if (fixtz && !!m[4] && !m[7])
        value = value.replace(/-/g, "/").replace(/T/, " ");
      value = new Date(value);
    } else
      continue;
    object[key] = value;
  }
  return object;
}
var fixtz = new Date("2019-01-01T00:00").getHours() || new Date("2019-07-01T00:00").getHours();
// node_modules/d3-fetch/src/blob.js
function responseBlob(response) {
  if (!response.ok)
    throw new Error(response.status + " " + response.statusText);
  return response.blob();
}
function blob_default(input, init2) {
  return fetch(input, init2).then(responseBlob);
}
// node_modules/d3-fetch/src/buffer.js
function responseArrayBuffer(response) {
  if (!response.ok)
    throw new Error(response.status + " " + response.statusText);
  return response.arrayBuffer();
}
function buffer_default(input, init2) {
  return fetch(input, init2).then(responseArrayBuffer);
}
// node_modules/d3-fetch/src/text.js
function responseText(response) {
  if (!response.ok)
    throw new Error(response.status + " " + response.statusText);
  return response.text();
}
function text_default3(input, init2) {
  return fetch(input, init2).then(responseText);
}

// node_modules/d3-fetch/src/dsv.js
function dsvParse(parse) {
  return function(input, init2, row) {
    if (arguments.length === 2 && typeof init2 === "function")
      row = init2, init2 = undefined;
    return text_default3(input, init2).then(function(response) {
      return parse(response, row);
    });
  };
}
function dsv(delimiter, input, init2, row) {
  if (arguments.length === 3 && typeof init2 === "function")
    row = init2, init2 = undefined;
  var format = dsv_default(delimiter);
  return text_default3(input, init2).then(function(response) {
    return format.parse(response, row);
  });
}
var csv2 = dsvParse(csvParse);
var tsv2 = dsvParse(tsvParse);
// node_modules/d3-fetch/src/image.js
function image_default(input, init2) {
  return new Promise(function(resolve, reject) {
    var image = new Image;
    for (var key in init2)
      image[key] = init2[key];
    image.onerror = reject;
    image.onload = function() {
      resolve(image);
    };
    image.src = input;
  });
}
// node_modules/d3-fetch/src/json.js
function responseJson(response) {
  if (!response.ok)
    throw new Error(response.status + " " + response.statusText);
  if (response.status === 204 || response.status === 205)
    return;
  return response.json();
}
function json_default(input, init2) {
  return fetch(input, init2).then(responseJson);
}
// node_modules/d3-fetch/src/xml.js
function parser(type2) {
  return (input, init2) => text_default3(input, init2).then((text) => new DOMParser().parseFromString(text, type2));
}
var xml_default = parser("application/xml");
var html = parser("text/html");
var svg = parser("image/svg+xml");
// node_modules/d3-force/src/center.js
function center_default(x, y) {
  var nodes, strength = 1;
  if (x == null)
    x = 0;
  if (y == null)
    y = 0;
  function force() {
    var i, n = nodes.length, node, sx = 0, sy = 0;
    for (i = 0;i < n; ++i) {
      node = nodes[i], sx += node.x, sy += node.y;
    }
    for (sx = (sx / n - x) * strength, sy = (sy / n - y) * strength, i = 0;i < n; ++i) {
      node = nodes[i], node.x -= sx, node.y -= sy;
    }
  }
  force.initialize = function(_) {
    nodes = _;
  };
  force.x = function(_) {
    return arguments.length ? (x = +_, force) : x;
  };
  force.y = function(_) {
    return arguments.length ? (y = +_, force) : y;
  };
  force.strength = function(_) {
    return arguments.length ? (strength = +_, force) : strength;
  };
  return force;
}
// node_modules/d3-quadtree/src/add.js
function add_default(d) {
  const x = +this._x.call(null, d), y = +this._y.call(null, d);
  return add(this.cover(x, y), x, y, d);
}
function add(tree, x, y, d) {
  if (isNaN(x) || isNaN(y))
    return tree;
  var parent, node = tree._root, leaf = { data: d }, x0 = tree._x0, y0 = tree._y0, x1 = tree._x1, y1 = tree._y1, xm, ym, xp, yp, right2, bottom2, i, j;
  if (!node)
    return tree._root = leaf, tree;
  while (node.length) {
    if (right2 = x >= (xm = (x0 + x1) / 2))
      x0 = xm;
    else
      x1 = xm;
    if (bottom2 = y >= (ym = (y0 + y1) / 2))
      y0 = ym;
    else
      y1 = ym;
    if (parent = node, !(node = node[i = bottom2 << 1 | right2]))
      return parent[i] = leaf, tree;
  }
  xp = +tree._x.call(null, node.data);
  yp = +tree._y.call(null, node.data);
  if (x === xp && y === yp)
    return leaf.next = node, parent ? parent[i] = leaf : tree._root = leaf, tree;
  do {
    parent = parent ? parent[i] = new Array(4) : tree._root = new Array(4);
    if (right2 = x >= (xm = (x0 + x1) / 2))
      x0 = xm;
    else
      x1 = xm;
    if (bottom2 = y >= (ym = (y0 + y1) / 2))
      y0 = ym;
    else
      y1 = ym;
  } while ((i = bottom2 << 1 | right2) === (j = (yp >= ym) << 1 | xp >= xm));
  return parent[j] = node, parent[i] = leaf, tree;
}
function addAll(data) {
  var d, i, n = data.length, x, y, xz = new Array(n), yz = new Array(n), x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (i = 0;i < n; ++i) {
    if (isNaN(x = +this._x.call(null, d = data[i])) || isNaN(y = +this._y.call(null, d)))
      continue;
    xz[i] = x;
    yz[i] = y;
    if (x < x0)
      x0 = x;
    if (x > x1)
      x1 = x;
    if (y < y0)
      y0 = y;
    if (y > y1)
      y1 = y;
  }
  if (x0 > x1 || y0 > y1)
    return this;
  this.cover(x0, y0).cover(x1, y1);
  for (i = 0;i < n; ++i) {
    add(this, xz[i], yz[i], data[i]);
  }
  return this;
}

// node_modules/d3-quadtree/src/cover.js
function cover_default(x, y) {
  if (isNaN(x = +x) || isNaN(y = +y))
    return this;
  var x0 = this._x0, y0 = this._y0, x1 = this._x1, y1 = this._y1;
  if (isNaN(x0)) {
    x1 = (x0 = Math.floor(x)) + 1;
    y1 = (y0 = Math.floor(y)) + 1;
  } else {
    var z = x1 - x0 || 1, node = this._root, parent, i;
    while (x0 > x || x >= x1 || y0 > y || y >= y1) {
      i = (y < y0) << 1 | x < x0;
      parent = new Array(4), parent[i] = node, node = parent, z *= 2;
      switch (i) {
        case 0:
          x1 = x0 + z, y1 = y0 + z;
          break;
        case 1:
          x0 = x1 - z, y1 = y0 + z;
          break;
        case 2:
          x1 = x0 + z, y0 = y1 - z;
          break;
        case 3:
          x0 = x1 - z, y0 = y1 - z;
          break;
      }
    }
    if (this._root && this._root.length)
      this._root = node;
  }
  this._x0 = x0;
  this._y0 = y0;
  this._x1 = x1;
  this._y1 = y1;
  return this;
}

// node_modules/d3-quadtree/src/data.js
function data_default2() {
  var data = [];
  this.visit(function(node) {
    if (!node.length)
      do
        data.push(node.data);
      while (node = node.next);
  });
  return data;
}

// node_modules/d3-quadtree/src/extent.js
function extent_default(_) {
  return arguments.length ? this.cover(+_[0][0], +_[0][1]).cover(+_[1][0], +_[1][1]) : isNaN(this._x0) ? undefined : [[this._x0, this._y0], [this._x1, this._y1]];
}

// node_modules/d3-quadtree/src/quad.js
function quad_default(node, x0, y0, x1, y1) {
  this.node = node;
  this.x0 = x0;
  this.y0 = y0;
  this.x1 = x1;
  this.y1 = y1;
}

// node_modules/d3-quadtree/src/find.js
function find_default(x, y, radius) {
  var data, x0 = this._x0, y0 = this._y0, x1, y1, x2, y2, x3 = this._x1, y3 = this._y1, quads = [], node = this._root, q, i;
  if (node)
    quads.push(new quad_default(node, x0, y0, x3, y3));
  if (radius == null)
    radius = Infinity;
  else {
    x0 = x - radius, y0 = y - radius;
    x3 = x + radius, y3 = y + radius;
    radius *= radius;
  }
  while (q = quads.pop()) {
    if (!(node = q.node) || (x1 = q.x0) > x3 || (y1 = q.y0) > y3 || (x2 = q.x1) < x0 || (y2 = q.y1) < y0)
      continue;
    if (node.length) {
      var xm = (x1 + x2) / 2, ym = (y1 + y2) / 2;
      quads.push(new quad_default(node[3], xm, ym, x2, y2), new quad_default(node[2], x1, ym, xm, y2), new quad_default(node[1], xm, y1, x2, ym), new quad_default(node[0], x1, y1, xm, ym));
      if (i = (y >= ym) << 1 | x >= xm) {
        q = quads[quads.length - 1];
        quads[quads.length - 1] = quads[quads.length - 1 - i];
        quads[quads.length - 1 - i] = q;
      }
    } else {
      var dx = x - +this._x.call(null, node.data), dy = y - +this._y.call(null, node.data), d2 = dx * dx + dy * dy;
      if (d2 < radius) {
        var d = Math.sqrt(radius = d2);
        x0 = x - d, y0 = y - d;
        x3 = x + d, y3 = y + d;
        data = node.data;
      }
    }
  }
  return data;
}

// node_modules/d3-quadtree/src/remove.js
function remove_default3(d) {
  if (isNaN(x = +this._x.call(null, d)) || isNaN(y = +this._y.call(null, d)))
    return this;
  var parent, node = this._root, retainer, previous, next, x0 = this._x0, y0 = this._y0, x1 = this._x1, y1 = this._y1, x, y, xm, ym, right2, bottom2, i, j;
  if (!node)
    return this;
  if (node.length)
    while (true) {
      if (right2 = x >= (xm = (x0 + x1) / 2))
        x0 = xm;
      else
        x1 = xm;
      if (bottom2 = y >= (ym = (y0 + y1) / 2))
        y0 = ym;
      else
        y1 = ym;
      if (!(parent = node, node = node[i = bottom2 << 1 | right2]))
        return this;
      if (!node.length)
        break;
      if (parent[i + 1 & 3] || parent[i + 2 & 3] || parent[i + 3 & 3])
        retainer = parent, j = i;
    }
  while (node.data !== d)
    if (!(previous = node, node = node.next))
      return this;
  if (next = node.next)
    delete node.next;
  if (previous)
    return next ? previous.next = next : delete previous.next, this;
  if (!parent)
    return this._root = next, this;
  next ? parent[i] = next : delete parent[i];
  if ((node = parent[0] || parent[1] || parent[2] || parent[3]) && node === (parent[3] || parent[2] || parent[1] || parent[0]) && !node.length) {
    if (retainer)
      retainer[j] = node;
    else
      this._root = node;
  }
  return this;
}
function removeAll(data) {
  for (var i = 0, n = data.length;i < n; ++i)
    this.remove(data[i]);
  return this;
}

// node_modules/d3-quadtree/src/root.js
function root_default() {
  return this._root;
}

// node_modules/d3-quadtree/src/size.js
function size_default2() {
  var size = 0;
  this.visit(function(node) {
    if (!node.length)
      do
        ++size;
      while (node = node.next);
  });
  return size;
}

// node_modules/d3-quadtree/src/visit.js
function visit_default(callback) {
  var quads = [], q, node = this._root, child, x0, y0, x1, y1;
  if (node)
    quads.push(new quad_default(node, this._x0, this._y0, this._x1, this._y1));
  while (q = quads.pop()) {
    if (!callback(node = q.node, x0 = q.x0, y0 = q.y0, x1 = q.x1, y1 = q.y1) && node.length) {
      var xm = (x0 + x1) / 2, ym = (y0 + y1) / 2;
      if (child = node[3])
        quads.push(new quad_default(child, xm, ym, x1, y1));
      if (child = node[2])
        quads.push(new quad_default(child, x0, ym, xm, y1));
      if (child = node[1])
        quads.push(new quad_default(child, xm, y0, x1, ym));
      if (child = node[0])
        quads.push(new quad_default(child, x0, y0, xm, ym));
    }
  }
  return this;
}

// node_modules/d3-quadtree/src/visitAfter.js
function visitAfter_default(callback) {
  var quads = [], next = [], q;
  if (this._root)
    quads.push(new quad_default(this._root, this._x0, this._y0, this._x1, this._y1));
  while (q = quads.pop()) {
    var node = q.node;
    if (node.length) {
      var child, x0 = q.x0, y0 = q.y0, x1 = q.x1, y1 = q.y1, xm = (x0 + x1) / 2, ym = (y0 + y1) / 2;
      if (child = node[0])
        quads.push(new quad_default(child, x0, y0, xm, ym));
      if (child = node[1])
        quads.push(new quad_default(child, xm, y0, x1, ym));
      if (child = node[2])
        quads.push(new quad_default(child, x0, ym, xm, y1));
      if (child = node[3])
        quads.push(new quad_default(child, xm, ym, x1, y1));
    }
    next.push(q);
  }
  while (q = next.pop()) {
    callback(q.node, q.x0, q.y0, q.x1, q.y1);
  }
  return this;
}

// node_modules/d3-quadtree/src/x.js
function defaultX2(d) {
  return d[0];
}
function x_default(_) {
  return arguments.length ? (this._x = _, this) : this._x;
}

// node_modules/d3-quadtree/src/y.js
function defaultY2(d) {
  return d[1];
}
function y_default(_) {
  return arguments.length ? (this._y = _, this) : this._y;
}

// node_modules/d3-quadtree/src/quadtree.js
function quadtree(nodes, x, y) {
  var tree = new Quadtree(x == null ? defaultX2 : x, y == null ? defaultY2 : y, NaN, NaN, NaN, NaN);
  return nodes == null ? tree : tree.addAll(nodes);
}
function Quadtree(x, y, x0, y0, x1, y1) {
  this._x = x;
  this._y = y;
  this._x0 = x0;
  this._y0 = y0;
  this._x1 = x1;
  this._y1 = y1;
  this._root = undefined;
}
function leaf_copy(leaf) {
  var copy = { data: leaf.data }, next = copy;
  while (leaf = leaf.next)
    next = next.next = { data: leaf.data };
  return copy;
}
var treeProto = quadtree.prototype = Quadtree.prototype;
treeProto.copy = function() {
  var copy = new Quadtree(this._x, this._y, this._x0, this._y0, this._x1, this._y1), node = this._root, nodes, child;
  if (!node)
    return copy;
  if (!node.length)
    return copy._root = leaf_copy(node), copy;
  nodes = [{ source: node, target: copy._root = new Array(4) }];
  while (node = nodes.pop()) {
    for (var i = 0;i < 4; ++i) {
      if (child = node.source[i]) {
        if (child.length)
          nodes.push({ source: child, target: node.target[i] = new Array(4) });
        else
          node.target[i] = leaf_copy(child);
      }
    }
  }
  return copy;
};
treeProto.add = add_default;
treeProto.addAll = addAll;
treeProto.cover = cover_default;
treeProto.data = data_default2;
treeProto.extent = extent_default;
treeProto.find = find_default;
treeProto.remove = remove_default3;
treeProto.removeAll = removeAll;
treeProto.root = root_default;
treeProto.size = size_default2;
treeProto.visit = visit_default;
treeProto.visitAfter = visitAfter_default;
treeProto.x = x_default;
treeProto.y = y_default;
// node_modules/d3-force/src/constant.js
function constant_default7(x) {
  return function() {
    return x;
  };
}

// node_modules/d3-force/src/jiggle.js
function jiggle_default(random) {
  return (random() - 0.5) * 0.000001;
}

// node_modules/d3-force/src/collide.js
function x(d) {
  return d.x + d.vx;
}
function y(d) {
  return d.y + d.vy;
}
function collide_default(radius) {
  var nodes, radii, random, strength = 1, iterations = 1;
  if (typeof radius !== "function")
    radius = constant_default7(radius == null ? 1 : +radius);
  function force() {
    var i, n = nodes.length, tree, node, xi, yi, ri, ri2;
    for (var k = 0;k < iterations; ++k) {
      tree = quadtree(nodes, x, y).visitAfter(prepare);
      for (i = 0;i < n; ++i) {
        node = nodes[i];
        ri = radii[node.index], ri2 = ri * ri;
        xi = node.x + node.vx;
        yi = node.y + node.vy;
        tree.visit(apply);
      }
    }
    function apply(quad, x0, y0, x1, y1) {
      var { data, r: rj } = quad, r = ri + rj;
      if (data) {
        if (data.index > node.index) {
          var x2 = xi - data.x - data.vx, y2 = yi - data.y - data.vy, l = x2 * x2 + y2 * y2;
          if (l < r * r) {
            if (x2 === 0)
              x2 = jiggle_default(random), l += x2 * x2;
            if (y2 === 0)
              y2 = jiggle_default(random), l += y2 * y2;
            l = (r - (l = Math.sqrt(l))) / l * strength;
            node.vx += (x2 *= l) * (r = (rj *= rj) / (ri2 + rj));
            node.vy += (y2 *= l) * r;
            data.vx -= x2 * (r = 1 - r);
            data.vy -= y2 * r;
          }
        }
        return;
      }
      return x0 > xi + r || x1 < xi - r || y0 > yi + r || y1 < yi - r;
    }
  }
  function prepare(quad) {
    if (quad.data)
      return quad.r = radii[quad.data.index];
    for (var i = quad.r = 0;i < 4; ++i) {
      if (quad[i] && quad[i].r > quad.r) {
        quad.r = quad[i].r;
      }
    }
  }
  function initialize() {
    if (!nodes)
      return;
    var i, n = nodes.length, node;
    radii = new Array(n);
    for (i = 0;i < n; ++i)
      node = nodes[i], radii[node.index] = +radius(node, i, nodes);
  }
  force.initialize = function(_nodes, _random) {
    nodes = _nodes;
    random = _random;
    initialize();
  };
  force.iterations = function(_) {
    return arguments.length ? (iterations = +_, force) : iterations;
  };
  force.strength = function(_) {
    return arguments.length ? (strength = +_, force) : strength;
  };
  force.radius = function(_) {
    return arguments.length ? (radius = typeof _ === "function" ? _ : constant_default7(+_), initialize(), force) : radius;
  };
  return force;
}
// node_modules/d3-force/src/link.js
function index2(d) {
  return d.index;
}
function find2(nodeById, nodeId) {
  var node = nodeById.get(nodeId);
  if (!node)
    throw new Error("node not found: " + nodeId);
  return node;
}
function link_default(links) {
  var id2 = index2, strength = defaultStrength, strengths, distance = constant_default7(30), distances, nodes, count2, bias, random, iterations = 1;
  if (links == null)
    links = [];
  function defaultStrength(link) {
    return 1 / Math.min(count2[link.source.index], count2[link.target.index]);
  }
  function force(alpha) {
    for (var k = 0, n = links.length;k < iterations; ++k) {
      for (var i = 0, link, source, target, x2, y2, l, b;i < n; ++i) {
        link = links[i], source = link.source, target = link.target;
        x2 = target.x + target.vx - source.x - source.vx || jiggle_default(random);
        y2 = target.y + target.vy - source.y - source.vy || jiggle_default(random);
        l = Math.sqrt(x2 * x2 + y2 * y2);
        l = (l - distances[i]) / l * alpha * strengths[i];
        x2 *= l, y2 *= l;
        target.vx -= x2 * (b = bias[i]);
        target.vy -= y2 * b;
        source.vx += x2 * (b = 1 - b);
        source.vy += y2 * b;
      }
    }
  }
  function initialize() {
    if (!nodes)
      return;
    var i, n = nodes.length, m = links.length, nodeById = new Map(nodes.map((d, i2) => [id2(d, i2, nodes), d])), link;
    for (i = 0, count2 = new Array(n);i < m; ++i) {
      link = links[i], link.index = i;
      if (typeof link.source !== "object")
        link.source = find2(nodeById, link.source);
      if (typeof link.target !== "object")
        link.target = find2(nodeById, link.target);
      count2[link.source.index] = (count2[link.source.index] || 0) + 1;
      count2[link.target.index] = (count2[link.target.index] || 0) + 1;
    }
    for (i = 0, bias = new Array(m);i < m; ++i) {
      link = links[i], bias[i] = count2[link.source.index] / (count2[link.source.index] + count2[link.target.index]);
    }
    strengths = new Array(m), initializeStrength();
    distances = new Array(m), initializeDistance();
  }
  function initializeStrength() {
    if (!nodes)
      return;
    for (var i = 0, n = links.length;i < n; ++i) {
      strengths[i] = +strength(links[i], i, links);
    }
  }
  function initializeDistance() {
    if (!nodes)
      return;
    for (var i = 0, n = links.length;i < n; ++i) {
      distances[i] = +distance(links[i], i, links);
    }
  }
  force.initialize = function(_nodes, _random) {
    nodes = _nodes;
    random = _random;
    initialize();
  };
  force.links = function(_) {
    return arguments.length ? (links = _, initialize(), force) : links;
  };
  force.id = function(_) {
    return arguments.length ? (id2 = _, force) : id2;
  };
  force.iterations = function(_) {
    return arguments.length ? (iterations = +_, force) : iterations;
  };
  force.strength = function(_) {
    return arguments.length ? (strength = typeof _ === "function" ? _ : constant_default7(+_), initializeStrength(), force) : strength;
  };
  force.distance = function(_) {
    return arguments.length ? (distance = typeof _ === "function" ? _ : constant_default7(+_), initializeDistance(), force) : distance;
  };
  return force;
}
// node_modules/d3-force/src/lcg.js
var a = 1664525;
var c = 1013904223;
var m = 4294967296;
function lcg_default() {
  let s = 1;
  return () => (s = (a * s + c) % m) / m;
}

// node_modules/d3-force/src/simulation.js
function x2(d) {
  return d.x;
}
function y2(d) {
  return d.y;
}
var initialRadius = 10;
var initialAngle = Math.PI * (3 - Math.sqrt(5));
function simulation_default(nodes) {
  var simulation, alpha = 1, alphaMin = 0.001, alphaDecay = 1 - Math.pow(alphaMin, 1 / 300), alphaTarget = 0, velocityDecay = 0.6, forces = new Map, stepper = timer(step), event = dispatch_default("tick", "end"), random = lcg_default();
  if (nodes == null)
    nodes = [];
  function step() {
    tick();
    event.call("tick", simulation);
    if (alpha < alphaMin) {
      stepper.stop();
      event.call("end", simulation);
    }
  }
  function tick(iterations) {
    var i, n = nodes.length, node;
    if (iterations === undefined)
      iterations = 1;
    for (var k = 0;k < iterations; ++k) {
      alpha += (alphaTarget - alpha) * alphaDecay;
      forces.forEach(function(force) {
        force(alpha);
      });
      for (i = 0;i < n; ++i) {
        node = nodes[i];
        if (node.fx == null)
          node.x += node.vx *= velocityDecay;
        else
          node.x = node.fx, node.vx = 0;
        if (node.fy == null)
          node.y += node.vy *= velocityDecay;
        else
          node.y = node.fy, node.vy = 0;
      }
    }
    return simulation;
  }
  function initializeNodes() {
    for (var i = 0, n = nodes.length, node;i < n; ++i) {
      node = nodes[i], node.index = i;
      if (node.fx != null)
        node.x = node.fx;
      if (node.fy != null)
        node.y = node.fy;
      if (isNaN(node.x) || isNaN(node.y)) {
        var radius = initialRadius * Math.sqrt(0.5 + i), angle = i * initialAngle;
        node.x = radius * Math.cos(angle);
        node.y = radius * Math.sin(angle);
      }
      if (isNaN(node.vx) || isNaN(node.vy)) {
        node.vx = node.vy = 0;
      }
    }
  }
  function initializeForce(force) {
    if (force.initialize)
      force.initialize(nodes, random);
    return force;
  }
  initializeNodes();
  return simulation = {
    tick,
    restart: function() {
      return stepper.restart(step), simulation;
    },
    stop: function() {
      return stepper.stop(), simulation;
    },
    nodes: function(_) {
      return arguments.length ? (nodes = _, initializeNodes(), forces.forEach(initializeForce), simulation) : nodes;
    },
    alpha: function(_) {
      return arguments.length ? (alpha = +_, simulation) : alpha;
    },
    alphaMin: function(_) {
      return arguments.length ? (alphaMin = +_, simulation) : alphaMin;
    },
    alphaDecay: function(_) {
      return arguments.length ? (alphaDecay = +_, simulation) : +alphaDecay;
    },
    alphaTarget: function(_) {
      return arguments.length ? (alphaTarget = +_, simulation) : alphaTarget;
    },
    velocityDecay: function(_) {
      return arguments.length ? (velocityDecay = 1 - _, simulation) : 1 - velocityDecay;
    },
    randomSource: function(_) {
      return arguments.length ? (random = _, forces.forEach(initializeForce), simulation) : random;
    },
    force: function(name, _) {
      return arguments.length > 1 ? (_ == null ? forces.delete(name) : forces.set(name, initializeForce(_)), simulation) : forces.get(name);
    },
    find: function(x3, y3, radius) {
      var i = 0, n = nodes.length, dx, dy, d2, node, closest;
      if (radius == null)
        radius = Infinity;
      else
        radius *= radius;
      for (i = 0;i < n; ++i) {
        node = nodes[i];
        dx = x3 - node.x;
        dy = y3 - node.y;
        d2 = dx * dx + dy * dy;
        if (d2 < radius)
          closest = node, radius = d2;
      }
      return closest;
    },
    on: function(name, _) {
      return arguments.length > 1 ? (event.on(name, _), simulation) : event.on(name);
    }
  };
}

// node_modules/d3-force/src/manyBody.js
function manyBody_default() {
  var nodes, node, random, alpha, strength = constant_default7(-30), strengths, distanceMin2 = 1, distanceMax2 = Infinity, theta2 = 0.81;
  function force(_) {
    var i, n = nodes.length, tree = quadtree(nodes, x2, y2).visitAfter(accumulate);
    for (alpha = _, i = 0;i < n; ++i)
      node = nodes[i], tree.visit(apply);
  }
  function initialize() {
    if (!nodes)
      return;
    var i, n = nodes.length, node2;
    strengths = new Array(n);
    for (i = 0;i < n; ++i)
      node2 = nodes[i], strengths[node2.index] = +strength(node2, i, nodes);
  }
  function accumulate(quad) {
    var strength2 = 0, q, c2, weight = 0, x3, y3, i;
    if (quad.length) {
      for (x3 = y3 = i = 0;i < 4; ++i) {
        if ((q = quad[i]) && (c2 = Math.abs(q.value))) {
          strength2 += q.value, weight += c2, x3 += c2 * q.x, y3 += c2 * q.y;
        }
      }
      quad.x = x3 / weight;
      quad.y = y3 / weight;
    } else {
      q = quad;
      q.x = q.data.x;
      q.y = q.data.y;
      do
        strength2 += strengths[q.data.index];
      while (q = q.next);
    }
    quad.value = strength2;
  }
  function apply(quad, x1, _, x22) {
    if (!quad.value)
      return true;
    var x3 = quad.x - node.x, y3 = quad.y - node.y, w = x22 - x1, l = x3 * x3 + y3 * y3;
    if (w * w / theta2 < l) {
      if (l < distanceMax2) {
        if (x3 === 0)
          x3 = jiggle_default(random), l += x3 * x3;
        if (y3 === 0)
          y3 = jiggle_default(random), l += y3 * y3;
        if (l < distanceMin2)
          l = Math.sqrt(distanceMin2 * l);
        node.vx += x3 * quad.value * alpha / l;
        node.vy += y3 * quad.value * alpha / l;
      }
      return true;
    } else if (quad.length || l >= distanceMax2)
      return;
    if (quad.data !== node || quad.next) {
      if (x3 === 0)
        x3 = jiggle_default(random), l += x3 * x3;
      if (y3 === 0)
        y3 = jiggle_default(random), l += y3 * y3;
      if (l < distanceMin2)
        l = Math.sqrt(distanceMin2 * l);
    }
    do
      if (quad.data !== node) {
        w = strengths[quad.data.index] * alpha / l;
        node.vx += x3 * w;
        node.vy += y3 * w;
      }
    while (quad = quad.next);
  }
  force.initialize = function(_nodes, _random) {
    nodes = _nodes;
    random = _random;
    initialize();
  };
  force.strength = function(_) {
    return arguments.length ? (strength = typeof _ === "function" ? _ : constant_default7(+_), initialize(), force) : strength;
  };
  force.distanceMin = function(_) {
    return arguments.length ? (distanceMin2 = _ * _, force) : Math.sqrt(distanceMin2);
  };
  force.distanceMax = function(_) {
    return arguments.length ? (distanceMax2 = _ * _, force) : Math.sqrt(distanceMax2);
  };
  force.theta = function(_) {
    return arguments.length ? (theta2 = _ * _, force) : Math.sqrt(theta2);
  };
  return force;
}
// node_modules/d3-force/src/radial.js
function radial_default(radius, x3, y3) {
  var nodes, strength = constant_default7(0.1), strengths, radiuses;
  if (typeof radius !== "function")
    radius = constant_default7(+radius);
  if (x3 == null)
    x3 = 0;
  if (y3 == null)
    y3 = 0;
  function force(alpha) {
    for (var i = 0, n = nodes.length;i < n; ++i) {
      var node = nodes[i], dx = node.x - x3 || 0.000001, dy = node.y - y3 || 0.000001, r = Math.sqrt(dx * dx + dy * dy), k = (radiuses[i] - r) * strengths[i] * alpha / r;
      node.vx += dx * k;
      node.vy += dy * k;
    }
  }
  function initialize() {
    if (!nodes)
      return;
    var i, n = nodes.length;
    strengths = new Array(n);
    radiuses = new Array(n);
    for (i = 0;i < n; ++i) {
      radiuses[i] = +radius(nodes[i], i, nodes);
      strengths[i] = isNaN(radiuses[i]) ? 0 : +strength(nodes[i], i, nodes);
    }
  }
  force.initialize = function(_) {
    nodes = _, initialize();
  };
  force.strength = function(_) {
    return arguments.length ? (strength = typeof _ === "function" ? _ : constant_default7(+_), initialize(), force) : strength;
  };
  force.radius = function(_) {
    return arguments.length ? (radius = typeof _ === "function" ? _ : constant_default7(+_), initialize(), force) : radius;
  };
  force.x = function(_) {
    return arguments.length ? (x3 = +_, force) : x3;
  };
  force.y = function(_) {
    return arguments.length ? (y3 = +_, force) : y3;
  };
  return force;
}
// node_modules/d3-force/src/x.js
function x_default2(x3) {
  var strength = constant_default7(0.1), nodes, strengths, xz;
  if (typeof x3 !== "function")
    x3 = constant_default7(x3 == null ? 0 : +x3);
  function force(alpha) {
    for (var i = 0, n = nodes.length, node;i < n; ++i) {
      node = nodes[i], node.vx += (xz[i] - node.x) * strengths[i] * alpha;
    }
  }
  function initialize() {
    if (!nodes)
      return;
    var i, n = nodes.length;
    strengths = new Array(n);
    xz = new Array(n);
    for (i = 0;i < n; ++i) {
      strengths[i] = isNaN(xz[i] = +x3(nodes[i], i, nodes)) ? 0 : +strength(nodes[i], i, nodes);
    }
  }
  force.initialize = function(_) {
    nodes = _;
    initialize();
  };
  force.strength = function(_) {
    return arguments.length ? (strength = typeof _ === "function" ? _ : constant_default7(+_), initialize(), force) : strength;
  };
  force.x = function(_) {
    return arguments.length ? (x3 = typeof _ === "function" ? _ : constant_default7(+_), initialize(), force) : x3;
  };
  return force;
}
// node_modules/d3-force/src/y.js
function y_default2(y3) {
  var strength = constant_default7(0.1), nodes, strengths, yz;
  if (typeof y3 !== "function")
    y3 = constant_default7(y3 == null ? 0 : +y3);
  function force(alpha) {
    for (var i = 0, n = nodes.length, node;i < n; ++i) {
      node = nodes[i], node.vy += (yz[i] - node.y) * strengths[i] * alpha;
    }
  }
  function initialize() {
    if (!nodes)
      return;
    var i, n = nodes.length;
    strengths = new Array(n);
    yz = new Array(n);
    for (i = 0;i < n; ++i) {
      strengths[i] = isNaN(yz[i] = +y3(nodes[i], i, nodes)) ? 0 : +strength(nodes[i], i, nodes);
    }
  }
  force.initialize = function(_) {
    nodes = _;
    initialize();
  };
  force.strength = function(_) {
    return arguments.length ? (strength = typeof _ === "function" ? _ : constant_default7(+_), initialize(), force) : strength;
  };
  force.y = function(_) {
    return arguments.length ? (y3 = typeof _ === "function" ? _ : constant_default7(+_), initialize(), force) : y3;
  };
  return force;
}
// node_modules/d3-format/src/formatDecimal.js
function formatDecimal_default(x3) {
  return Math.abs(x3 = Math.round(x3)) >= 1000000000000000000000 ? x3.toLocaleString("en").replace(/,/g, "") : x3.toString(10);
}
function formatDecimalParts(x3, p) {
  if (!isFinite(x3) || x3 === 0)
    return null;
  var i = (x3 = p ? x3.toExponential(p - 1) : x3.toExponential()).indexOf("e"), coefficient = x3.slice(0, i);
  return [
    coefficient.length > 1 ? coefficient[0] + coefficient.slice(2) : coefficient,
    +x3.slice(i + 1)
  ];
}

// node_modules/d3-format/src/exponent.js
function exponent_default(x3) {
  return x3 = formatDecimalParts(Math.abs(x3)), x3 ? x3[1] : NaN;
}

// node_modules/d3-format/src/formatGroup.js
function formatGroup_default(grouping, thousands) {
  return function(value, width) {
    var i = value.length, t = [], j = 0, g = grouping[0], length3 = 0;
    while (i > 0 && g > 0) {
      if (length3 + g + 1 > width)
        g = Math.max(1, width - length3);
      t.push(value.substring(i -= g, i + g));
      if ((length3 += g + 1) > width)
        break;
      g = grouping[j = (j + 1) % grouping.length];
    }
    return t.reverse().join(thousands);
  };
}

// node_modules/d3-format/src/formatNumerals.js
function formatNumerals_default(numerals) {
  return function(value) {
    return value.replace(/[0-9]/g, function(i) {
      return numerals[+i];
    });
  };
}

// node_modules/d3-format/src/formatSpecifier.js
var re = /^(?:(.)?([<>=^]))?([+\-( ])?([$#])?(0)?(\d+)?(,)?(\.\d+)?(~)?([a-z%])?$/i;
function formatSpecifier(specifier) {
  if (!(match = re.exec(specifier)))
    throw new Error("invalid format: " + specifier);
  var match;
  return new FormatSpecifier({
    fill: match[1],
    align: match[2],
    sign: match[3],
    symbol: match[4],
    zero: match[5],
    width: match[6],
    comma: match[7],
    precision: match[8] && match[8].slice(1),
    trim: match[9],
    type: match[10]
  });
}
formatSpecifier.prototype = FormatSpecifier.prototype;
function FormatSpecifier(specifier) {
  this.fill = specifier.fill === undefined ? " " : specifier.fill + "";
  this.align = specifier.align === undefined ? ">" : specifier.align + "";
  this.sign = specifier.sign === undefined ? "-" : specifier.sign + "";
  this.symbol = specifier.symbol === undefined ? "" : specifier.symbol + "";
  this.zero = !!specifier.zero;
  this.width = specifier.width === undefined ? undefined : +specifier.width;
  this.comma = !!specifier.comma;
  this.precision = specifier.precision === undefined ? undefined : +specifier.precision;
  this.trim = !!specifier.trim;
  this.type = specifier.type === undefined ? "" : specifier.type + "";
}
FormatSpecifier.prototype.toString = function() {
  return this.fill + this.align + this.sign + this.symbol + (this.zero ? "0" : "") + (this.width === undefined ? "" : Math.max(1, this.width | 0)) + (this.comma ? "," : "") + (this.precision === undefined ? "" : "." + Math.max(0, this.precision | 0)) + (this.trim ? "~" : "") + this.type;
};

// node_modules/d3-format/src/formatTrim.js
function formatTrim_default(s) {
  out:
    for (var n = s.length, i = 1, i0 = -1, i1;i < n; ++i) {
      switch (s[i]) {
        case ".":
          i0 = i1 = i;
          break;
        case "0":
          if (i0 === 0)
            i0 = i;
          i1 = i;
          break;
        default:
          if (!+s[i])
            break out;
          if (i0 > 0)
            i0 = 0;
          break;
      }
    }
  return i0 > 0 ? s.slice(0, i0) + s.slice(i1 + 1) : s;
}

// node_modules/d3-format/src/formatPrefixAuto.js
var prefixExponent;
function formatPrefixAuto_default(x3, p) {
  var d = formatDecimalParts(x3, p);
  if (!d)
    return prefixExponent = undefined, x3.toPrecision(p);
  var coefficient = d[0], exponent2 = d[1], i = exponent2 - (prefixExponent = Math.max(-8, Math.min(8, Math.floor(exponent2 / 3))) * 3) + 1, n = coefficient.length;
  return i === n ? coefficient : i > n ? coefficient + new Array(i - n + 1).join("0") : i > 0 ? coefficient.slice(0, i) + "." + coefficient.slice(i) : "0." + new Array(1 - i).join("0") + formatDecimalParts(x3, Math.max(0, p + i - 1))[0];
}

// node_modules/d3-format/src/formatRounded.js
function formatRounded_default(x3, p) {
  var d = formatDecimalParts(x3, p);
  if (!d)
    return x3 + "";
  var coefficient = d[0], exponent2 = d[1];
  return exponent2 < 0 ? "0." + new Array(-exponent2).join("0") + coefficient : coefficient.length > exponent2 + 1 ? coefficient.slice(0, exponent2 + 1) + "." + coefficient.slice(exponent2 + 1) : coefficient + new Array(exponent2 - coefficient.length + 2).join("0");
}

// node_modules/d3-format/src/formatTypes.js
var formatTypes_default = {
  "%": (x3, p) => (x3 * 100).toFixed(p),
  b: (x3) => Math.round(x3).toString(2),
  c: (x3) => x3 + "",
  d: formatDecimal_default,
  e: (x3, p) => x3.toExponential(p),
  f: (x3, p) => x3.toFixed(p),
  g: (x3, p) => x3.toPrecision(p),
  o: (x3) => Math.round(x3).toString(8),
  p: (x3, p) => formatRounded_default(x3 * 100, p),
  r: formatRounded_default,
  s: formatPrefixAuto_default,
  X: (x3) => Math.round(x3).toString(16).toUpperCase(),
  x: (x3) => Math.round(x3).toString(16)
};

// node_modules/d3-format/src/identity.js
function identity_default2(x3) {
  return x3;
}

// node_modules/d3-format/src/locale.js
var map3 = Array.prototype.map;
var prefixes = ["y", "z", "a", "f", "p", "n", "µ", "m", "", "k", "M", "G", "T", "P", "E", "Z", "Y"];
function locale_default(locale) {
  var group2 = locale.grouping === undefined || locale.thousands === undefined ? identity_default2 : formatGroup_default(map3.call(locale.grouping, Number), locale.thousands + ""), currencyPrefix = locale.currency === undefined ? "" : locale.currency[0] + "", currencySuffix = locale.currency === undefined ? "" : locale.currency[1] + "", decimal = locale.decimal === undefined ? "." : locale.decimal + "", numerals = locale.numerals === undefined ? identity_default2 : formatNumerals_default(map3.call(locale.numerals, String)), percent = locale.percent === undefined ? "%" : locale.percent + "", minus = locale.minus === undefined ? "−" : locale.minus + "", nan = locale.nan === undefined ? "NaN" : locale.nan + "";
  function newFormat(specifier, options) {
    specifier = formatSpecifier(specifier);
    var { fill, align, sign, symbol, zero: zero3, width, comma, precision, trim, type: type2 } = specifier;
    if (type2 === "n")
      comma = true, type2 = "g";
    else if (!formatTypes_default[type2])
      precision === undefined && (precision = 12), trim = true, type2 = "g";
    if (zero3 || fill === "0" && align === "=")
      zero3 = true, fill = "0", align = "=";
    var prefix = (options && options.prefix !== undefined ? options.prefix : "") + (symbol === "$" ? currencyPrefix : symbol === "#" && /[boxX]/.test(type2) ? "0" + type2.toLowerCase() : ""), suffix = (symbol === "$" ? currencySuffix : /[%p]/.test(type2) ? percent : "") + (options && options.suffix !== undefined ? options.suffix : "");
    var formatType = formatTypes_default[type2], maybeSuffix = /[defgprs%]/.test(type2);
    precision = precision === undefined ? 6 : /[gprs]/.test(type2) ? Math.max(1, Math.min(21, precision)) : Math.max(0, Math.min(20, precision));
    function format(value) {
      var valuePrefix = prefix, valueSuffix = suffix, i, n, c2;
      if (type2 === "c") {
        valueSuffix = formatType(value) + valueSuffix;
        value = "";
      } else {
        value = +value;
        var valueNegative = value < 0 || 1 / value < 0;
        value = isNaN(value) ? nan : formatType(Math.abs(value), precision);
        if (trim)
          value = formatTrim_default(value);
        if (valueNegative && +value === 0 && sign !== "+")
          valueNegative = false;
        valuePrefix = (valueNegative ? sign === "(" ? sign : minus : sign === "-" || sign === "(" ? "" : sign) + valuePrefix;
        valueSuffix = (type2 === "s" && !isNaN(value) && prefixExponent !== undefined ? prefixes[8 + prefixExponent / 3] : "") + valueSuffix + (valueNegative && sign === "(" ? ")" : "");
        if (maybeSuffix) {
          i = -1, n = value.length;
          while (++i < n) {
            if (c2 = value.charCodeAt(i), 48 > c2 || c2 > 57) {
              valueSuffix = (c2 === 46 ? decimal + value.slice(i + 1) : value.slice(i)) + valueSuffix;
              value = value.slice(0, i);
              break;
            }
          }
        }
      }
      if (comma && !zero3)
        value = group2(value, Infinity);
      var length3 = valuePrefix.length + value.length + valueSuffix.length, padding = length3 < width ? new Array(width - length3 + 1).join(fill) : "";
      if (comma && zero3)
        value = group2(padding + value, padding.length ? width - valueSuffix.length : Infinity), padding = "";
      switch (align) {
        case "<":
          value = valuePrefix + value + valueSuffix + padding;
          break;
        case "=":
          value = valuePrefix + padding + value + valueSuffix;
          break;
        case "^":
          value = padding.slice(0, length3 = padding.length >> 1) + valuePrefix + value + valueSuffix + padding.slice(length3);
          break;
        default:
          value = padding + valuePrefix + value + valueSuffix;
          break;
      }
      return numerals(value);
    }
    format.toString = function() {
      return specifier + "";
    };
    return format;
  }
  function formatPrefix(specifier, value) {
    var e = Math.max(-8, Math.min(8, Math.floor(exponent_default(value) / 3))) * 3, k = Math.pow(10, -e), f = newFormat((specifier = formatSpecifier(specifier), specifier.type = "f", specifier), { suffix: prefixes[8 + e / 3] });
    return function(value2) {
      return f(k * value2);
    };
  }
  return {
    format: newFormat,
    formatPrefix
  };
}

// node_modules/d3-format/src/defaultLocale.js
var locale;
var format;
var formatPrefix;
defaultLocale({
  thousands: ",",
  grouping: [3],
  currency: ["$", ""]
});
function defaultLocale(definition) {
  locale = locale_default(definition);
  format = locale.format;
  formatPrefix = locale.formatPrefix;
  return locale;
}
// node_modules/d3-format/src/precisionFixed.js
function precisionFixed_default(step) {
  return Math.max(0, -exponent_default(Math.abs(step)));
}
// node_modules/d3-format/src/precisionPrefix.js
function precisionPrefix_default(step, value) {
  return Math.max(0, Math.max(-8, Math.min(8, Math.floor(exponent_default(value) / 3))) * 3 - exponent_default(Math.abs(step)));
}
// node_modules/d3-format/src/precisionRound.js
function precisionRound_default(step, max4) {
  step = Math.abs(step), max4 = Math.abs(max4) - step;
  return Math.max(0, exponent_default(max4) - exponent_default(step)) + 1;
}
// node_modules/d3-geo/src/math.js
var epsilon8 = 0.000001;
var epsilon22 = 0.000000000001;
var pi4 = Math.PI;
var halfPi3 = pi4 / 2;
var quarterPi = pi4 / 4;
var tau5 = pi4 * 2;
var degrees3 = 180 / pi4;
var radians2 = pi4 / 180;
var abs3 = Math.abs;
var atan = Math.atan;
var atan2 = Math.atan2;
var cos2 = Math.cos;
var ceil = Math.ceil;
var exp = Math.exp;
var hypot = Math.hypot;
var log = Math.log;
var pow2 = Math.pow;
var sin2 = Math.sin;
var sign = Math.sign || function(x3) {
  return x3 > 0 ? 1 : x3 < 0 ? -1 : 0;
};
var sqrt = Math.sqrt;
var tan = Math.tan;
function acos(x3) {
  return x3 > 1 ? 0 : x3 < -1 ? pi4 : Math.acos(x3);
}
function asin(x3) {
  return x3 > 1 ? halfPi3 : x3 < -1 ? -halfPi3 : Math.asin(x3);
}
function haversin(x3) {
  return (x3 = sin2(x3 / 2)) * x3;
}

// node_modules/d3-geo/src/noop.js
function noop2() {}

// node_modules/d3-geo/src/stream.js
function streamGeometry(geometry, stream) {
  if (geometry && streamGeometryType.hasOwnProperty(geometry.type)) {
    streamGeometryType[geometry.type](geometry, stream);
  }
}
var streamObjectType = {
  Feature: function(object, stream) {
    streamGeometry(object.geometry, stream);
  },
  FeatureCollection: function(object, stream) {
    var features = object.features, i = -1, n = features.length;
    while (++i < n)
      streamGeometry(features[i].geometry, stream);
  }
};
var streamGeometryType = {
  Sphere: function(object, stream) {
    stream.sphere();
  },
  Point: function(object, stream) {
    object = object.coordinates;
    stream.point(object[0], object[1], object[2]);
  },
  MultiPoint: function(object, stream) {
    var coordinates = object.coordinates, i = -1, n = coordinates.length;
    while (++i < n)
      object = coordinates[i], stream.point(object[0], object[1], object[2]);
  },
  LineString: function(object, stream) {
    streamLine(object.coordinates, stream, 0);
  },
  MultiLineString: function(object, stream) {
    var coordinates = object.coordinates, i = -1, n = coordinates.length;
    while (++i < n)
      streamLine(coordinates[i], stream, 0);
  },
  Polygon: function(object, stream) {
    streamPolygon(object.coordinates, stream);
  },
  MultiPolygon: function(object, stream) {
    var coordinates = object.coordinates, i = -1, n = coordinates.length;
    while (++i < n)
      streamPolygon(coordinates[i], stream);
  },
  GeometryCollection: function(object, stream) {
    var geometries = object.geometries, i = -1, n = geometries.length;
    while (++i < n)
      streamGeometry(geometries[i], stream);
  }
};
function streamLine(coordinates, stream, closed) {
  var i = -1, n = coordinates.length - closed, coordinate;
  stream.lineStart();
  while (++i < n)
    coordinate = coordinates[i], stream.point(coordinate[0], coordinate[1], coordinate[2]);
  stream.lineEnd();
}
function streamPolygon(coordinates, stream) {
  var i = -1, n = coordinates.length;
  stream.polygonStart();
  while (++i < n)
    streamLine(coordinates[i], stream, 1);
  stream.polygonEnd();
}
function stream_default(object, stream) {
  if (object && streamObjectType.hasOwnProperty(object.type)) {
    streamObjectType[object.type](object, stream);
  } else {
    streamGeometry(object, stream);
  }
}

// node_modules/d3-geo/src/area.js
var areaRingSum = new Adder;
var areaSum = new Adder;
var lambda00;
var phi00;
var lambda0;
var cosPhi0;
var sinPhi0;
var areaStream = {
  point: noop2,
  lineStart: noop2,
  lineEnd: noop2,
  polygonStart: function() {
    areaRingSum = new Adder;
    areaStream.lineStart = areaRingStart;
    areaStream.lineEnd = areaRingEnd;
  },
  polygonEnd: function() {
    var areaRing = +areaRingSum;
    areaSum.add(areaRing < 0 ? tau5 + areaRing : areaRing);
    this.lineStart = this.lineEnd = this.point = noop2;
  },
  sphere: function() {
    areaSum.add(tau5);
  }
};
function areaRingStart() {
  areaStream.point = areaPointFirst;
}
function areaRingEnd() {
  areaPoint(lambda00, phi00);
}
function areaPointFirst(lambda, phi) {
  areaStream.point = areaPoint;
  lambda00 = lambda, phi00 = phi;
  lambda *= radians2, phi *= radians2;
  lambda0 = lambda, cosPhi0 = cos2(phi = phi / 2 + quarterPi), sinPhi0 = sin2(phi);
}
function areaPoint(lambda, phi) {
  lambda *= radians2, phi *= radians2;
  phi = phi / 2 + quarterPi;
  var dLambda = lambda - lambda0, sdLambda = dLambda >= 0 ? 1 : -1, adLambda = sdLambda * dLambda, cosPhi = cos2(phi), sinPhi = sin2(phi), k = sinPhi0 * sinPhi, u4 = cosPhi0 * cosPhi + k * cos2(adLambda), v2 = k * sdLambda * sin2(adLambda);
  areaRingSum.add(atan2(v2, u4));
  lambda0 = lambda, cosPhi0 = cosPhi, sinPhi0 = sinPhi;
}
function area_default2(object) {
  areaSum = new Adder;
  stream_default(object, areaStream);
  return areaSum * 2;
}
// node_modules/d3-geo/src/cartesian.js
function spherical(cartesian) {
  return [atan2(cartesian[1], cartesian[0]), asin(cartesian[2])];
}
function cartesian(spherical2) {
  var lambda = spherical2[0], phi = spherical2[1], cosPhi = cos2(phi);
  return [cosPhi * cos2(lambda), cosPhi * sin2(lambda), sin2(phi)];
}
function cartesianDot(a2, b) {
  return a2[0] * b[0] + a2[1] * b[1] + a2[2] * b[2];
}
function cartesianCross(a2, b) {
  return [a2[1] * b[2] - a2[2] * b[1], a2[2] * b[0] - a2[0] * b[2], a2[0] * b[1] - a2[1] * b[0]];
}
function cartesianAddInPlace(a2, b) {
  a2[0] += b[0], a2[1] += b[1], a2[2] += b[2];
}
function cartesianScale(vector, k) {
  return [vector[0] * k, vector[1] * k, vector[2] * k];
}
function cartesianNormalizeInPlace(d) {
  var l = sqrt(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
  d[0] /= l, d[1] /= l, d[2] /= l;
}

// node_modules/d3-geo/src/bounds.js
var lambda02;
var phi0;
var lambda1;
var phi1;
var lambda2;
var lambda002;
var phi002;
var p0;
var deltaSum;
var ranges;
var range3;
var boundsStream = {
  point: boundsPoint,
  lineStart: boundsLineStart,
  lineEnd: boundsLineEnd,
  polygonStart: function() {
    boundsStream.point = boundsRingPoint;
    boundsStream.lineStart = boundsRingStart;
    boundsStream.lineEnd = boundsRingEnd;
    deltaSum = new Adder;
    areaStream.polygonStart();
  },
  polygonEnd: function() {
    areaStream.polygonEnd();
    boundsStream.point = boundsPoint;
    boundsStream.lineStart = boundsLineStart;
    boundsStream.lineEnd = boundsLineEnd;
    if (areaRingSum < 0)
      lambda02 = -(lambda1 = 180), phi0 = -(phi1 = 90);
    else if (deltaSum > epsilon8)
      phi1 = 90;
    else if (deltaSum < -epsilon8)
      phi0 = -90;
    range3[0] = lambda02, range3[1] = lambda1;
  },
  sphere: function() {
    lambda02 = -(lambda1 = 180), phi0 = -(phi1 = 90);
  }
};
function boundsPoint(lambda, phi) {
  ranges.push(range3 = [lambda02 = lambda, lambda1 = lambda]);
  if (phi < phi0)
    phi0 = phi;
  if (phi > phi1)
    phi1 = phi;
}
function linePoint(lambda, phi) {
  var p = cartesian([lambda * radians2, phi * radians2]);
  if (p0) {
    var normal = cartesianCross(p0, p), equatorial = [normal[1], -normal[0], 0], inflection = cartesianCross(equatorial, normal);
    cartesianNormalizeInPlace(inflection);
    inflection = spherical(inflection);
    var delta = lambda - lambda2, sign2 = delta > 0 ? 1 : -1, lambdai = inflection[0] * degrees3 * sign2, phii, antimeridian = abs3(delta) > 180;
    if (antimeridian ^ (sign2 * lambda2 < lambdai && lambdai < sign2 * lambda)) {
      phii = inflection[1] * degrees3;
      if (phii > phi1)
        phi1 = phii;
    } else if (lambdai = (lambdai + 360) % 360 - 180, antimeridian ^ (sign2 * lambda2 < lambdai && lambdai < sign2 * lambda)) {
      phii = -inflection[1] * degrees3;
      if (phii < phi0)
        phi0 = phii;
    } else {
      if (phi < phi0)
        phi0 = phi;
      if (phi > phi1)
        phi1 = phi;
    }
    if (antimeridian) {
      if (lambda < lambda2) {
        if (angle(lambda02, lambda) > angle(lambda02, lambda1))
          lambda1 = lambda;
      } else {
        if (angle(lambda, lambda1) > angle(lambda02, lambda1))
          lambda02 = lambda;
      }
    } else {
      if (lambda1 >= lambda02) {
        if (lambda < lambda02)
          lambda02 = lambda;
        if (lambda > lambda1)
          lambda1 = lambda;
      } else {
        if (lambda > lambda2) {
          if (angle(lambda02, lambda) > angle(lambda02, lambda1))
            lambda1 = lambda;
        } else {
          if (angle(lambda, lambda1) > angle(lambda02, lambda1))
            lambda02 = lambda;
        }
      }
    }
  } else {
    ranges.push(range3 = [lambda02 = lambda, lambda1 = lambda]);
  }
  if (phi < phi0)
    phi0 = phi;
  if (phi > phi1)
    phi1 = phi;
  p0 = p, lambda2 = lambda;
}
function boundsLineStart() {
  boundsStream.point = linePoint;
}
function boundsLineEnd() {
  range3[0] = lambda02, range3[1] = lambda1;
  boundsStream.point = boundsPoint;
  p0 = null;
}
function boundsRingPoint(lambda, phi) {
  if (p0) {
    var delta = lambda - lambda2;
    deltaSum.add(abs3(delta) > 180 ? delta + (delta > 0 ? 360 : -360) : delta);
  } else {
    lambda002 = lambda, phi002 = phi;
  }
  areaStream.point(lambda, phi);
  linePoint(lambda, phi);
}
function boundsRingStart() {
  areaStream.lineStart();
}
function boundsRingEnd() {
  boundsRingPoint(lambda002, phi002);
  areaStream.lineEnd();
  if (abs3(deltaSum) > epsilon8)
    lambda02 = -(lambda1 = 180);
  range3[0] = lambda02, range3[1] = lambda1;
  p0 = null;
}
function angle(lambda03, lambda12) {
  return (lambda12 -= lambda03) < 0 ? lambda12 + 360 : lambda12;
}
function rangeCompare(a2, b) {
  return a2[0] - b[0];
}
function rangeContains(range4, x3) {
  return range4[0] <= range4[1] ? range4[0] <= x3 && x3 <= range4[1] : x3 < range4[0] || range4[1] < x3;
}
function bounds_default(feature) {
  var i, n, a2, b, merged, deltaMax, delta;
  phi1 = lambda1 = -(lambda02 = phi0 = Infinity);
  ranges = [];
  stream_default(feature, boundsStream);
  if (n = ranges.length) {
    ranges.sort(rangeCompare);
    for (i = 1, a2 = ranges[0], merged = [a2];i < n; ++i) {
      b = ranges[i];
      if (rangeContains(a2, b[0]) || rangeContains(a2, b[1])) {
        if (angle(a2[0], b[1]) > angle(a2[0], a2[1]))
          a2[1] = b[1];
        if (angle(b[0], a2[1]) > angle(a2[0], a2[1]))
          a2[0] = b[0];
      } else {
        merged.push(a2 = b);
      }
    }
    for (deltaMax = -Infinity, n = merged.length - 1, i = 0, a2 = merged[n];i <= n; a2 = b, ++i) {
      b = merged[i];
      if ((delta = angle(a2[1], b[0])) > deltaMax)
        deltaMax = delta, lambda02 = b[0], lambda1 = a2[1];
    }
  }
  ranges = range3 = null;
  return lambda02 === Infinity || phi0 === Infinity ? [[NaN, NaN], [NaN, NaN]] : [[lambda02, phi0], [lambda1, phi1]];
}
// node_modules/d3-geo/src/centroid.js
var W0;
var W1;
var X0;
var Y0;
var Z0;
var X1;
var Y1;
var Z1;
var X2;
var Y2;
var Z2;
var lambda003;
var phi003;
var x0;
var y0;
var z0;
var centroidStream = {
  sphere: noop2,
  point: centroidPoint,
  lineStart: centroidLineStart,
  lineEnd: centroidLineEnd,
  polygonStart: function() {
    centroidStream.lineStart = centroidRingStart;
    centroidStream.lineEnd = centroidRingEnd;
  },
  polygonEnd: function() {
    centroidStream.lineStart = centroidLineStart;
    centroidStream.lineEnd = centroidLineEnd;
  }
};
function centroidPoint(lambda, phi) {
  lambda *= radians2, phi *= radians2;
  var cosPhi = cos2(phi);
  centroidPointCartesian(cosPhi * cos2(lambda), cosPhi * sin2(lambda), sin2(phi));
}
function centroidPointCartesian(x3, y3, z) {
  ++W0;
  X0 += (x3 - X0) / W0;
  Y0 += (y3 - Y0) / W0;
  Z0 += (z - Z0) / W0;
}
function centroidLineStart() {
  centroidStream.point = centroidLinePointFirst;
}
function centroidLinePointFirst(lambda, phi) {
  lambda *= radians2, phi *= radians2;
  var cosPhi = cos2(phi);
  x0 = cosPhi * cos2(lambda);
  y0 = cosPhi * sin2(lambda);
  z0 = sin2(phi);
  centroidStream.point = centroidLinePoint;
  centroidPointCartesian(x0, y0, z0);
}
function centroidLinePoint(lambda, phi) {
  lambda *= radians2, phi *= radians2;
  var cosPhi = cos2(phi), x3 = cosPhi * cos2(lambda), y3 = cosPhi * sin2(lambda), z = sin2(phi), w = atan2(sqrt((w = y0 * z - z0 * y3) * w + (w = z0 * x3 - x0 * z) * w + (w = x0 * y3 - y0 * x3) * w), x0 * x3 + y0 * y3 + z0 * z);
  W1 += w;
  X1 += w * (x0 + (x0 = x3));
  Y1 += w * (y0 + (y0 = y3));
  Z1 += w * (z0 + (z0 = z));
  centroidPointCartesian(x0, y0, z0);
}
function centroidLineEnd() {
  centroidStream.point = centroidPoint;
}
function centroidRingStart() {
  centroidStream.point = centroidRingPointFirst;
}
function centroidRingEnd() {
  centroidRingPoint(lambda003, phi003);
  centroidStream.point = centroidPoint;
}
function centroidRingPointFirst(lambda, phi) {
  lambda003 = lambda, phi003 = phi;
  lambda *= radians2, phi *= radians2;
  centroidStream.point = centroidRingPoint;
  var cosPhi = cos2(phi);
  x0 = cosPhi * cos2(lambda);
  y0 = cosPhi * sin2(lambda);
  z0 = sin2(phi);
  centroidPointCartesian(x0, y0, z0);
}
function centroidRingPoint(lambda, phi) {
  lambda *= radians2, phi *= radians2;
  var cosPhi = cos2(phi), x3 = cosPhi * cos2(lambda), y3 = cosPhi * sin2(lambda), z = sin2(phi), cx = y0 * z - z0 * y3, cy = z0 * x3 - x0 * z, cz = x0 * y3 - y0 * x3, m2 = hypot(cx, cy, cz), w = asin(m2), v2 = m2 && -w / m2;
  X2.add(v2 * cx);
  Y2.add(v2 * cy);
  Z2.add(v2 * cz);
  W1 += w;
  X1 += w * (x0 + (x0 = x3));
  Y1 += w * (y0 + (y0 = y3));
  Z1 += w * (z0 + (z0 = z));
  centroidPointCartesian(x0, y0, z0);
}
function centroid_default(object) {
  W0 = W1 = X0 = Y0 = Z0 = X1 = Y1 = Z1 = 0;
  X2 = new Adder;
  Y2 = new Adder;
  Z2 = new Adder;
  stream_default(object, centroidStream);
  var x3 = +X2, y3 = +Y2, z = +Z2, m2 = hypot(x3, y3, z);
  if (m2 < epsilon22) {
    x3 = X1, y3 = Y1, z = Z1;
    if (W1 < epsilon8)
      x3 = X0, y3 = Y0, z = Z0;
    m2 = hypot(x3, y3, z);
    if (m2 < epsilon22)
      return [NaN, NaN];
  }
  return [atan2(y3, x3) * degrees3, asin(z / m2) * degrees3];
}
// node_modules/d3-geo/src/constant.js
function constant_default8(x3) {
  return function() {
    return x3;
  };
}

// node_modules/d3-geo/src/compose.js
function compose_default(a2, b) {
  function compose(x3, y3) {
    return x3 = a2(x3, y3), b(x3[0], x3[1]);
  }
  if (a2.invert && b.invert)
    compose.invert = function(x3, y3) {
      return x3 = b.invert(x3, y3), x3 && a2.invert(x3[0], x3[1]);
    };
  return compose;
}

// node_modules/d3-geo/src/rotation.js
function rotationIdentity(lambda, phi) {
  if (abs3(lambda) > pi4)
    lambda -= Math.round(lambda / tau5) * tau5;
  return [lambda, phi];
}
rotationIdentity.invert = rotationIdentity;
function rotateRadians(deltaLambda, deltaPhi, deltaGamma) {
  return (deltaLambda %= tau5) ? deltaPhi || deltaGamma ? compose_default(rotationLambda(deltaLambda), rotationPhiGamma(deltaPhi, deltaGamma)) : rotationLambda(deltaLambda) : deltaPhi || deltaGamma ? rotationPhiGamma(deltaPhi, deltaGamma) : rotationIdentity;
}
function forwardRotationLambda(deltaLambda) {
  return function(lambda, phi) {
    lambda += deltaLambda;
    if (abs3(lambda) > pi4)
      lambda -= Math.round(lambda / tau5) * tau5;
    return [lambda, phi];
  };
}
function rotationLambda(deltaLambda) {
  var rotation = forwardRotationLambda(deltaLambda);
  rotation.invert = forwardRotationLambda(-deltaLambda);
  return rotation;
}
function rotationPhiGamma(deltaPhi, deltaGamma) {
  var cosDeltaPhi = cos2(deltaPhi), sinDeltaPhi = sin2(deltaPhi), cosDeltaGamma = cos2(deltaGamma), sinDeltaGamma = sin2(deltaGamma);
  function rotation(lambda, phi) {
    var cosPhi = cos2(phi), x3 = cos2(lambda) * cosPhi, y3 = sin2(lambda) * cosPhi, z = sin2(phi), k = z * cosDeltaPhi + x3 * sinDeltaPhi;
    return [
      atan2(y3 * cosDeltaGamma - k * sinDeltaGamma, x3 * cosDeltaPhi - z * sinDeltaPhi),
      asin(k * cosDeltaGamma + y3 * sinDeltaGamma)
    ];
  }
  rotation.invert = function(lambda, phi) {
    var cosPhi = cos2(phi), x3 = cos2(lambda) * cosPhi, y3 = sin2(lambda) * cosPhi, z = sin2(phi), k = z * cosDeltaGamma - y3 * sinDeltaGamma;
    return [
      atan2(y3 * cosDeltaGamma + z * sinDeltaGamma, x3 * cosDeltaPhi + k * sinDeltaPhi),
      asin(k * cosDeltaPhi - x3 * sinDeltaPhi)
    ];
  };
  return rotation;
}
function rotation_default(rotate) {
  rotate = rotateRadians(rotate[0] * radians2, rotate[1] * radians2, rotate.length > 2 ? rotate[2] * radians2 : 0);
  function forward(coordinates) {
    coordinates = rotate(coordinates[0] * radians2, coordinates[1] * radians2);
    return coordinates[0] *= degrees3, coordinates[1] *= degrees3, coordinates;
  }
  forward.invert = function(coordinates) {
    coordinates = rotate.invert(coordinates[0] * radians2, coordinates[1] * radians2);
    return coordinates[0] *= degrees3, coordinates[1] *= degrees3, coordinates;
  };
  return forward;
}

// node_modules/d3-geo/src/circle.js
function circleStream(stream, radius, delta, direction, t02, t12) {
  if (!delta)
    return;
  var cosRadius = cos2(radius), sinRadius = sin2(radius), step = direction * delta;
  if (t02 == null) {
    t02 = radius + direction * tau5;
    t12 = radius - step / 2;
  } else {
    t02 = circleRadius(cosRadius, t02);
    t12 = circleRadius(cosRadius, t12);
    if (direction > 0 ? t02 < t12 : t02 > t12)
      t02 += direction * tau5;
  }
  for (var point, t = t02;direction > 0 ? t > t12 : t < t12; t -= step) {
    point = spherical([cosRadius, -sinRadius * cos2(t), -sinRadius * sin2(t)]);
    stream.point(point[0], point[1]);
  }
}
function circleRadius(cosRadius, point) {
  point = cartesian(point), point[0] -= cosRadius;
  cartesianNormalizeInPlace(point);
  var radius = acos(-point[1]);
  return ((-point[2] < 0 ? -radius : radius) + tau5 - epsilon8) % tau5;
}
function circle_default() {
  var center2 = constant_default8([0, 0]), radius = constant_default8(90), precision = constant_default8(2), ring, rotate, stream = { point };
  function point(x3, y3) {
    ring.push(x3 = rotate(x3, y3));
    x3[0] *= degrees3, x3[1] *= degrees3;
  }
  function circle() {
    var c2 = center2.apply(this, arguments), r = radius.apply(this, arguments) * radians2, p = precision.apply(this, arguments) * radians2;
    ring = [];
    rotate = rotateRadians(-c2[0] * radians2, -c2[1] * radians2, 0).invert;
    circleStream(stream, r, p, 1);
    c2 = { type: "Polygon", coordinates: [ring] };
    ring = rotate = null;
    return c2;
  }
  circle.center = function(_) {
    return arguments.length ? (center2 = typeof _ === "function" ? _ : constant_default8([+_[0], +_[1]]), circle) : center2;
  };
  circle.radius = function(_) {
    return arguments.length ? (radius = typeof _ === "function" ? _ : constant_default8(+_), circle) : radius;
  };
  circle.precision = function(_) {
    return arguments.length ? (precision = typeof _ === "function" ? _ : constant_default8(+_), circle) : precision;
  };
  return circle;
}
// node_modules/d3-geo/src/clip/buffer.js
function buffer_default2() {
  var lines = [], line;
  return {
    point: function(x3, y3, m2) {
      line.push([x3, y3, m2]);
    },
    lineStart: function() {
      lines.push(line = []);
    },
    lineEnd: noop2,
    rejoin: function() {
      if (lines.length > 1)
        lines.push(lines.pop().concat(lines.shift()));
    },
    result: function() {
      var result = lines;
      lines = [];
      line = null;
      return result;
    }
  };
}

// node_modules/d3-geo/src/pointEqual.js
function pointEqual_default(a2, b) {
  return abs3(a2[0] - b[0]) < epsilon8 && abs3(a2[1] - b[1]) < epsilon8;
}

// node_modules/d3-geo/src/clip/rejoin.js
function Intersection(point, points, other, entry) {
  this.x = point;
  this.z = points;
  this.o = other;
  this.e = entry;
  this.v = false;
  this.n = this.p = null;
}
function rejoin_default(segments, compareIntersection, startInside, interpolate, stream) {
  var subject = [], clip = [], i, n;
  segments.forEach(function(segment) {
    if ((n2 = segment.length - 1) <= 0)
      return;
    var n2, p02 = segment[0], p1 = segment[n2], x3;
    if (pointEqual_default(p02, p1)) {
      if (!p02[2] && !p1[2]) {
        stream.lineStart();
        for (i = 0;i < n2; ++i)
          stream.point((p02 = segment[i])[0], p02[1]);
        stream.lineEnd();
        return;
      }
      p1[0] += 2 * epsilon8;
    }
    subject.push(x3 = new Intersection(p02, segment, null, true));
    clip.push(x3.o = new Intersection(p02, null, x3, false));
    subject.push(x3 = new Intersection(p1, segment, null, false));
    clip.push(x3.o = new Intersection(p1, null, x3, true));
  });
  if (!subject.length)
    return;
  clip.sort(compareIntersection);
  link(subject);
  link(clip);
  for (i = 0, n = clip.length;i < n; ++i) {
    clip[i].e = startInside = !startInside;
  }
  var start2 = subject[0], points, point;
  while (true) {
    var current = start2, isSubject = true;
    while (current.v)
      if ((current = current.n) === start2)
        return;
    points = current.z;
    stream.lineStart();
    do {
      current.v = current.o.v = true;
      if (current.e) {
        if (isSubject) {
          for (i = 0, n = points.length;i < n; ++i)
            stream.point((point = points[i])[0], point[1]);
        } else {
          interpolate(current.x, current.n.x, 1, stream);
        }
        current = current.n;
      } else {
        if (isSubject) {
          points = current.p.z;
          for (i = points.length - 1;i >= 0; --i)
            stream.point((point = points[i])[0], point[1]);
        } else {
          interpolate(current.x, current.p.x, -1, stream);
        }
        current = current.p;
      }
      current = current.o;
      points = current.z;
      isSubject = !isSubject;
    } while (!current.v);
    stream.lineEnd();
  }
}
function link(array4) {
  if (!(n = array4.length))
    return;
  var n, i = 0, a2 = array4[0], b;
  while (++i < n) {
    a2.n = b = array4[i];
    b.p = a2;
    a2 = b;
  }
  a2.n = b = array4[0];
  b.p = a2;
}

// node_modules/d3-geo/src/polygonContains.js
function longitude(point) {
  return abs3(point[0]) <= pi4 ? point[0] : sign(point[0]) * ((abs3(point[0]) + pi4) % tau5 - pi4);
}
function polygonContains_default(polygon, point) {
  var lambda = longitude(point), phi = point[1], sinPhi = sin2(phi), normal = [sin2(lambda), -cos2(lambda), 0], angle2 = 0, winding = 0;
  var sum3 = new Adder;
  if (sinPhi === 1)
    phi = halfPi3 + epsilon8;
  else if (sinPhi === -1)
    phi = -halfPi3 - epsilon8;
  for (var i = 0, n = polygon.length;i < n; ++i) {
    if (!(m2 = (ring = polygon[i]).length))
      continue;
    var ring, m2, point0 = ring[m2 - 1], lambda03 = longitude(point0), phi02 = point0[1] / 2 + quarterPi, sinPhi02 = sin2(phi02), cosPhi02 = cos2(phi02);
    for (var j = 0;j < m2; ++j, lambda03 = lambda12, sinPhi02 = sinPhi1, cosPhi02 = cosPhi1, point0 = point1) {
      var point1 = ring[j], lambda12 = longitude(point1), phi12 = point1[1] / 2 + quarterPi, sinPhi1 = sin2(phi12), cosPhi1 = cos2(phi12), delta = lambda12 - lambda03, sign2 = delta >= 0 ? 1 : -1, absDelta = sign2 * delta, antimeridian = absDelta > pi4, k = sinPhi02 * sinPhi1;
      sum3.add(atan2(k * sign2 * sin2(absDelta), cosPhi02 * cosPhi1 + k * cos2(absDelta)));
      angle2 += antimeridian ? delta + sign2 * tau5 : delta;
      if (antimeridian ^ lambda03 >= lambda ^ lambda12 >= lambda) {
        var arc = cartesianCross(cartesian(point0), cartesian(point1));
        cartesianNormalizeInPlace(arc);
        var intersection2 = cartesianCross(normal, arc);
        cartesianNormalizeInPlace(intersection2);
        var phiArc = (antimeridian ^ delta >= 0 ? -1 : 1) * asin(intersection2[2]);
        if (phi > phiArc || phi === phiArc && (arc[0] || arc[1])) {
          winding += antimeridian ^ delta >= 0 ? 1 : -1;
        }
      }
    }
  }
  return (angle2 < -epsilon8 || angle2 < epsilon8 && sum3 < -epsilon22) ^ winding & 1;
}

// node_modules/d3-geo/src/clip/index.js
function clip_default(pointVisible, clipLine, interpolate, start2) {
  return function(sink) {
    var line = clipLine(sink), ringBuffer = buffer_default2(), ringSink = clipLine(ringBuffer), polygonStarted = false, polygon, segments, ring;
    var clip = {
      point,
      lineStart,
      lineEnd,
      polygonStart: function() {
        clip.point = pointRing;
        clip.lineStart = ringStart;
        clip.lineEnd = ringEnd;
        segments = [];
        polygon = [];
      },
      polygonEnd: function() {
        clip.point = point;
        clip.lineStart = lineStart;
        clip.lineEnd = lineEnd;
        segments = merge(segments);
        var startInside = polygonContains_default(polygon, start2);
        if (segments.length) {
          if (!polygonStarted)
            sink.polygonStart(), polygonStarted = true;
          rejoin_default(segments, compareIntersection, startInside, interpolate, sink);
        } else if (startInside) {
          if (!polygonStarted)
            sink.polygonStart(), polygonStarted = true;
          sink.lineStart();
          interpolate(null, null, 1, sink);
          sink.lineEnd();
        }
        if (polygonStarted)
          sink.polygonEnd(), polygonStarted = false;
        segments = polygon = null;
      },
      sphere: function() {
        sink.polygonStart();
        sink.lineStart();
        interpolate(null, null, 1, sink);
        sink.lineEnd();
        sink.polygonEnd();
      }
    };
    function point(lambda, phi) {
      if (pointVisible(lambda, phi))
        sink.point(lambda, phi);
    }
    function pointLine(lambda, phi) {
      line.point(lambda, phi);
    }
    function lineStart() {
      clip.point = pointLine;
      line.lineStart();
    }
    function lineEnd() {
      clip.point = point;
      line.lineEnd();
    }
    function pointRing(lambda, phi) {
      ring.push([lambda, phi]);
      ringSink.point(lambda, phi);
    }
    function ringStart() {
      ringSink.lineStart();
      ring = [];
    }
    function ringEnd() {
      pointRing(ring[0][0], ring[0][1]);
      ringSink.lineEnd();
      var clean = ringSink.clean(), ringSegments = ringBuffer.result(), i, n = ringSegments.length, m2, segment, point2;
      ring.pop();
      polygon.push(ring);
      ring = null;
      if (!n)
        return;
      if (clean & 1) {
        segment = ringSegments[0];
        if ((m2 = segment.length - 1) > 0) {
          if (!polygonStarted)
            sink.polygonStart(), polygonStarted = true;
          sink.lineStart();
          for (i = 0;i < m2; ++i)
            sink.point((point2 = segment[i])[0], point2[1]);
          sink.lineEnd();
        }
        return;
      }
      if (n > 1 && clean & 2)
        ringSegments.push(ringSegments.pop().concat(ringSegments.shift()));
      segments.push(ringSegments.filter(validSegment));
    }
    return clip;
  };
}
function validSegment(segment) {
  return segment.length > 1;
}
function compareIntersection(a2, b) {
  return ((a2 = a2.x)[0] < 0 ? a2[1] - halfPi3 - epsilon8 : halfPi3 - a2[1]) - ((b = b.x)[0] < 0 ? b[1] - halfPi3 - epsilon8 : halfPi3 - b[1]);
}

// node_modules/d3-geo/src/clip/antimeridian.js
var antimeridian_default = clip_default(function() {
  return true;
}, clipAntimeridianLine, clipAntimeridianInterpolate, [-pi4, -halfPi3]);
function clipAntimeridianLine(stream) {
  var lambda03 = NaN, phi02 = NaN, sign0 = NaN, clean;
  return {
    lineStart: function() {
      stream.lineStart();
      clean = 1;
    },
    point: function(lambda12, phi12) {
      var sign1 = lambda12 > 0 ? pi4 : -pi4, delta = abs3(lambda12 - lambda03);
      if (abs3(delta - pi4) < epsilon8) {
        stream.point(lambda03, phi02 = (phi02 + phi12) / 2 > 0 ? halfPi3 : -halfPi3);
        stream.point(sign0, phi02);
        stream.lineEnd();
        stream.lineStart();
        stream.point(sign1, phi02);
        stream.point(lambda12, phi02);
        clean = 0;
      } else if (sign0 !== sign1 && delta >= pi4) {
        if (abs3(lambda03 - sign0) < epsilon8)
          lambda03 -= sign0 * epsilon8;
        if (abs3(lambda12 - sign1) < epsilon8)
          lambda12 -= sign1 * epsilon8;
        phi02 = clipAntimeridianIntersect(lambda03, phi02, lambda12, phi12);
        stream.point(sign0, phi02);
        stream.lineEnd();
        stream.lineStart();
        stream.point(sign1, phi02);
        clean = 0;
      }
      stream.point(lambda03 = lambda12, phi02 = phi12);
      sign0 = sign1;
    },
    lineEnd: function() {
      stream.lineEnd();
      lambda03 = phi02 = NaN;
    },
    clean: function() {
      return 2 - clean;
    }
  };
}
function clipAntimeridianIntersect(lambda03, phi02, lambda12, phi12) {
  var cosPhi02, cosPhi1, sinLambda0Lambda1 = sin2(lambda03 - lambda12);
  return abs3(sinLambda0Lambda1) > epsilon8 ? atan((sin2(phi02) * (cosPhi1 = cos2(phi12)) * sin2(lambda12) - sin2(phi12) * (cosPhi02 = cos2(phi02)) * sin2(lambda03)) / (cosPhi02 * cosPhi1 * sinLambda0Lambda1)) : (phi02 + phi12) / 2;
}
function clipAntimeridianInterpolate(from, to, direction, stream) {
  var phi;
  if (from == null) {
    phi = direction * halfPi3;
    stream.point(-pi4, phi);
    stream.point(0, phi);
    stream.point(pi4, phi);
    stream.point(pi4, 0);
    stream.point(pi4, -phi);
    stream.point(0, -phi);
    stream.point(-pi4, -phi);
    stream.point(-pi4, 0);
    stream.point(-pi4, phi);
  } else if (abs3(from[0] - to[0]) > epsilon8) {
    var lambda = from[0] < to[0] ? pi4 : -pi4;
    phi = direction * lambda / 2;
    stream.point(-lambda, phi);
    stream.point(0, phi);
    stream.point(lambda, phi);
  } else {
    stream.point(to[0], to[1]);
  }
}
// node_modules/d3-geo/src/clip/circle.js
function circle_default2(radius) {
  var cr = cos2(radius), delta = 2 * radians2, smallRadius = cr > 0, notHemisphere = abs3(cr) > epsilon8;
  function interpolate(from, to, direction, stream) {
    circleStream(stream, radius, delta, direction, from, to);
  }
  function visible(lambda, phi) {
    return cos2(lambda) * cos2(phi) > cr;
  }
  function clipLine(stream) {
    var point0, c0, v0, v00, clean;
    return {
      lineStart: function() {
        v00 = v0 = false;
        clean = 1;
      },
      point: function(lambda, phi) {
        var point1 = [lambda, phi], point2, v2 = visible(lambda, phi), c2 = smallRadius ? v2 ? 0 : code(lambda, phi) : v2 ? code(lambda + (lambda < 0 ? pi4 : -pi4), phi) : 0;
        if (!point0 && (v00 = v0 = v2))
          stream.lineStart();
        if (v2 !== v0) {
          point2 = intersect(point0, point1);
          if (!point2 || pointEqual_default(point0, point2) || pointEqual_default(point1, point2))
            point1[2] = 1;
        }
        if (v2 !== v0) {
          clean = 0;
          if (v2) {
            stream.lineStart();
            point2 = intersect(point1, point0);
            stream.point(point2[0], point2[1]);
          } else {
            point2 = intersect(point0, point1);
            stream.point(point2[0], point2[1], 2);
            stream.lineEnd();
          }
          point0 = point2;
        } else if (notHemisphere && point0 && smallRadius ^ v2) {
          var t;
          if (!(c2 & c0) && (t = intersect(point1, point0, true))) {
            clean = 0;
            if (smallRadius) {
              stream.lineStart();
              stream.point(t[0][0], t[0][1]);
              stream.point(t[1][0], t[1][1]);
              stream.lineEnd();
            } else {
              stream.point(t[1][0], t[1][1]);
              stream.lineEnd();
              stream.lineStart();
              stream.point(t[0][0], t[0][1], 3);
            }
          }
        }
        if (v2 && (!point0 || !pointEqual_default(point0, point1))) {
          stream.point(point1[0], point1[1]);
        }
        point0 = point1, v0 = v2, c0 = c2;
      },
      lineEnd: function() {
        if (v0)
          stream.lineEnd();
        point0 = null;
      },
      clean: function() {
        return clean | (v00 && v0) << 1;
      }
    };
  }
  function intersect(a2, b, two) {
    var pa = cartesian(a2), pb = cartesian(b);
    var n1 = [1, 0, 0], n2 = cartesianCross(pa, pb), n2n2 = cartesianDot(n2, n2), n1n2 = n2[0], determinant = n2n2 - n1n2 * n1n2;
    if (!determinant)
      return !two && a2;
    var c1 = cr * n2n2 / determinant, c2 = -cr * n1n2 / determinant, n1xn2 = cartesianCross(n1, n2), A2 = cartesianScale(n1, c1), B3 = cartesianScale(n2, c2);
    cartesianAddInPlace(A2, B3);
    var u4 = n1xn2, w = cartesianDot(A2, u4), uu = cartesianDot(u4, u4), t22 = w * w - uu * (cartesianDot(A2, A2) - 1);
    if (t22 < 0)
      return;
    var t = sqrt(t22), q = cartesianScale(u4, (-w - t) / uu);
    cartesianAddInPlace(q, A2);
    q = spherical(q);
    if (!two)
      return q;
    var lambda03 = a2[0], lambda12 = b[0], phi02 = a2[1], phi12 = b[1], z;
    if (lambda12 < lambda03)
      z = lambda03, lambda03 = lambda12, lambda12 = z;
    var delta2 = lambda12 - lambda03, polar = abs3(delta2 - pi4) < epsilon8, meridian = polar || delta2 < epsilon8;
    if (!polar && phi12 < phi02)
      z = phi02, phi02 = phi12, phi12 = z;
    if (meridian ? polar ? phi02 + phi12 > 0 ^ q[1] < (abs3(q[0] - lambda03) < epsilon8 ? phi02 : phi12) : phi02 <= q[1] && q[1] <= phi12 : delta2 > pi4 ^ (lambda03 <= q[0] && q[0] <= lambda12)) {
      var q1 = cartesianScale(u4, (-w + t) / uu);
      cartesianAddInPlace(q1, A2);
      return [q, spherical(q1)];
    }
  }
  function code(lambda, phi) {
    var r = smallRadius ? radius : pi4 - radius, code2 = 0;
    if (lambda < -r)
      code2 |= 1;
    else if (lambda > r)
      code2 |= 2;
    if (phi < -r)
      code2 |= 4;
    else if (phi > r)
      code2 |= 8;
    return code2;
  }
  return clip_default(visible, clipLine, interpolate, smallRadius ? [0, -radius] : [-pi4, radius - pi4]);
}
// node_modules/d3-geo/src/clip/line.js
function line_default(a2, b, x02, y02, x1, y1) {
  var ax = a2[0], ay = a2[1], bx = b[0], by = b[1], t02 = 0, t12 = 1, dx = bx - ax, dy = by - ay, r;
  r = x02 - ax;
  if (!dx && r > 0)
    return;
  r /= dx;
  if (dx < 0) {
    if (r < t02)
      return;
    if (r < t12)
      t12 = r;
  } else if (dx > 0) {
    if (r > t12)
      return;
    if (r > t02)
      t02 = r;
  }
  r = x1 - ax;
  if (!dx && r < 0)
    return;
  r /= dx;
  if (dx < 0) {
    if (r > t12)
      return;
    if (r > t02)
      t02 = r;
  } else if (dx > 0) {
    if (r < t02)
      return;
    if (r < t12)
      t12 = r;
  }
  r = y02 - ay;
  if (!dy && r > 0)
    return;
  r /= dy;
  if (dy < 0) {
    if (r < t02)
      return;
    if (r < t12)
      t12 = r;
  } else if (dy > 0) {
    if (r > t12)
      return;
    if (r > t02)
      t02 = r;
  }
  r = y1 - ay;
  if (!dy && r < 0)
    return;
  r /= dy;
  if (dy < 0) {
    if (r > t12)
      return;
    if (r > t02)
      t02 = r;
  } else if (dy > 0) {
    if (r < t02)
      return;
    if (r < t12)
      t12 = r;
  }
  if (t02 > 0)
    a2[0] = ax + t02 * dx, a2[1] = ay + t02 * dy;
  if (t12 < 1)
    b[0] = ax + t12 * dx, b[1] = ay + t12 * dy;
  return true;
}

// node_modules/d3-geo/src/clip/rectangle.js
var clipMax = 1e9;
var clipMin = -clipMax;
function clipRectangle(x02, y02, x1, y1) {
  function visible(x3, y3) {
    return x02 <= x3 && x3 <= x1 && y02 <= y3 && y3 <= y1;
  }
  function interpolate(from, to, direction, stream) {
    var a2 = 0, a1 = 0;
    if (from == null || (a2 = corner(from, direction)) !== (a1 = corner(to, direction)) || comparePoint(from, to) < 0 ^ direction > 0) {
      do
        stream.point(a2 === 0 || a2 === 3 ? x02 : x1, a2 > 1 ? y1 : y02);
      while ((a2 = (a2 + direction + 4) % 4) !== a1);
    } else {
      stream.point(to[0], to[1]);
    }
  }
  function corner(p, direction) {
    return abs3(p[0] - x02) < epsilon8 ? direction > 0 ? 0 : 3 : abs3(p[0] - x1) < epsilon8 ? direction > 0 ? 2 : 1 : abs3(p[1] - y02) < epsilon8 ? direction > 0 ? 1 : 0 : direction > 0 ? 3 : 2;
  }
  function compareIntersection2(a2, b) {
    return comparePoint(a2.x, b.x);
  }
  function comparePoint(a2, b) {
    var ca3 = corner(a2, 1), cb = corner(b, 1);
    return ca3 !== cb ? ca3 - cb : ca3 === 0 ? b[1] - a2[1] : ca3 === 1 ? a2[0] - b[0] : ca3 === 2 ? a2[1] - b[1] : b[0] - a2[0];
  }
  return function(stream) {
    var activeStream = stream, bufferStream = buffer_default2(), segments, polygon, ring, x__, y__, v__, x_, y_, v_, first, clean;
    var clipStream = {
      point,
      lineStart,
      lineEnd,
      polygonStart,
      polygonEnd
    };
    function point(x3, y3) {
      if (visible(x3, y3))
        activeStream.point(x3, y3);
    }
    function polygonInside() {
      var winding = 0;
      for (var i = 0, n = polygon.length;i < n; ++i) {
        for (var ring2 = polygon[i], j = 1, m2 = ring2.length, point2 = ring2[0], a0, a1, b02 = point2[0], b12 = point2[1];j < m2; ++j) {
          a0 = b02, a1 = b12, point2 = ring2[j], b02 = point2[0], b12 = point2[1];
          if (a1 <= y1) {
            if (b12 > y1 && (b02 - a0) * (y1 - a1) > (b12 - a1) * (x02 - a0))
              ++winding;
          } else {
            if (b12 <= y1 && (b02 - a0) * (y1 - a1) < (b12 - a1) * (x02 - a0))
              --winding;
          }
        }
      }
      return winding;
    }
    function polygonStart() {
      activeStream = bufferStream, segments = [], polygon = [], clean = true;
    }
    function polygonEnd() {
      var startInside = polygonInside(), cleanInside = clean && startInside, visible2 = (segments = merge(segments)).length;
      if (cleanInside || visible2) {
        stream.polygonStart();
        if (cleanInside) {
          stream.lineStart();
          interpolate(null, null, 1, stream);
          stream.lineEnd();
        }
        if (visible2) {
          rejoin_default(segments, compareIntersection2, startInside, interpolate, stream);
        }
        stream.polygonEnd();
      }
      activeStream = stream, segments = polygon = ring = null;
    }
    function lineStart() {
      clipStream.point = linePoint2;
      if (polygon)
        polygon.push(ring = []);
      first = true;
      v_ = false;
      x_ = y_ = NaN;
    }
    function lineEnd() {
      if (segments) {
        linePoint2(x__, y__);
        if (v__ && v_)
          bufferStream.rejoin();
        segments.push(bufferStream.result());
      }
      clipStream.point = point;
      if (v_)
        activeStream.lineEnd();
    }
    function linePoint2(x3, y3) {
      var v2 = visible(x3, y3);
      if (polygon)
        ring.push([x3, y3]);
      if (first) {
        x__ = x3, y__ = y3, v__ = v2;
        first = false;
        if (v2) {
          activeStream.lineStart();
          activeStream.point(x3, y3);
        }
      } else {
        if (v2 && v_)
          activeStream.point(x3, y3);
        else {
          var a2 = [x_ = Math.max(clipMin, Math.min(clipMax, x_)), y_ = Math.max(clipMin, Math.min(clipMax, y_))], b = [x3 = Math.max(clipMin, Math.min(clipMax, x3)), y3 = Math.max(clipMin, Math.min(clipMax, y3))];
          if (line_default(a2, b, x02, y02, x1, y1)) {
            if (!v_) {
              activeStream.lineStart();
              activeStream.point(a2[0], a2[1]);
            }
            activeStream.point(b[0], b[1]);
            if (!v2)
              activeStream.lineEnd();
            clean = false;
          } else if (v2) {
            activeStream.lineStart();
            activeStream.point(x3, y3);
            clean = false;
          }
        }
      }
      x_ = x3, y_ = y3, v_ = v2;
    }
    return clipStream;
  };
}

// node_modules/d3-geo/src/clip/extent.js
function extent_default2() {
  var x02 = 0, y02 = 0, x1 = 960, y1 = 500, cache, cacheStream, clip;
  return clip = {
    stream: function(stream) {
      return cache && cacheStream === stream ? cache : cache = clipRectangle(x02, y02, x1, y1)(cacheStream = stream);
    },
    extent: function(_) {
      return arguments.length ? (x02 = +_[0][0], y02 = +_[0][1], x1 = +_[1][0], y1 = +_[1][1], cache = cacheStream = null, clip) : [[x02, y02], [x1, y1]];
    }
  };
}
// node_modules/d3-geo/src/length.js
var lengthSum;
var lambda03;
var sinPhi02;
var cosPhi02;
var lengthStream = {
  sphere: noop2,
  point: noop2,
  lineStart: lengthLineStart,
  lineEnd: noop2,
  polygonStart: noop2,
  polygonEnd: noop2
};
function lengthLineStart() {
  lengthStream.point = lengthPointFirst;
  lengthStream.lineEnd = lengthLineEnd;
}
function lengthLineEnd() {
  lengthStream.point = lengthStream.lineEnd = noop2;
}
function lengthPointFirst(lambda, phi) {
  lambda *= radians2, phi *= radians2;
  lambda03 = lambda, sinPhi02 = sin2(phi), cosPhi02 = cos2(phi);
  lengthStream.point = lengthPoint;
}
function lengthPoint(lambda, phi) {
  lambda *= radians2, phi *= radians2;
  var sinPhi = sin2(phi), cosPhi = cos2(phi), delta = abs3(lambda - lambda03), cosDelta = cos2(delta), sinDelta = sin2(delta), x3 = cosPhi * sinDelta, y3 = cosPhi02 * sinPhi - sinPhi02 * cosPhi * cosDelta, z = sinPhi02 * sinPhi + cosPhi02 * cosPhi * cosDelta;
  lengthSum.add(atan2(sqrt(x3 * x3 + y3 * y3), z));
  lambda03 = lambda, sinPhi02 = sinPhi, cosPhi02 = cosPhi;
}
function length_default(object) {
  lengthSum = new Adder;
  stream_default(object, lengthStream);
  return +lengthSum;
}

// node_modules/d3-geo/src/distance.js
var coordinates = [null, null];
var object = { type: "LineString", coordinates };
function distance_default(a2, b) {
  coordinates[0] = a2;
  coordinates[1] = b;
  return length_default(object);
}

// node_modules/d3-geo/src/contains.js
var containsObjectType = {
  Feature: function(object2, point) {
    return containsGeometry(object2.geometry, point);
  },
  FeatureCollection: function(object2, point) {
    var features = object2.features, i = -1, n = features.length;
    while (++i < n)
      if (containsGeometry(features[i].geometry, point))
        return true;
    return false;
  }
};
var containsGeometryType = {
  Sphere: function() {
    return true;
  },
  Point: function(object2, point) {
    return containsPoint(object2.coordinates, point);
  },
  MultiPoint: function(object2, point) {
    var coordinates2 = object2.coordinates, i = -1, n = coordinates2.length;
    while (++i < n)
      if (containsPoint(coordinates2[i], point))
        return true;
    return false;
  },
  LineString: function(object2, point) {
    return containsLine(object2.coordinates, point);
  },
  MultiLineString: function(object2, point) {
    var coordinates2 = object2.coordinates, i = -1, n = coordinates2.length;
    while (++i < n)
      if (containsLine(coordinates2[i], point))
        return true;
    return false;
  },
  Polygon: function(object2, point) {
    return containsPolygon(object2.coordinates, point);
  },
  MultiPolygon: function(object2, point) {
    var coordinates2 = object2.coordinates, i = -1, n = coordinates2.length;
    while (++i < n)
      if (containsPolygon(coordinates2[i], point))
        return true;
    return false;
  },
  GeometryCollection: function(object2, point) {
    var geometries = object2.geometries, i = -1, n = geometries.length;
    while (++i < n)
      if (containsGeometry(geometries[i], point))
        return true;
    return false;
  }
};
function containsGeometry(geometry, point) {
  return geometry && containsGeometryType.hasOwnProperty(geometry.type) ? containsGeometryType[geometry.type](geometry, point) : false;
}
function containsPoint(coordinates2, point) {
  return distance_default(coordinates2, point) === 0;
}
function containsLine(coordinates2, point) {
  var ao, bo, ab4;
  for (var i = 0, n = coordinates2.length;i < n; i++) {
    bo = distance_default(coordinates2[i], point);
    if (bo === 0)
      return true;
    if (i > 0) {
      ab4 = distance_default(coordinates2[i], coordinates2[i - 1]);
      if (ab4 > 0 && ao <= ab4 && bo <= ab4 && (ao + bo - ab4) * (1 - Math.pow((ao - bo) / ab4, 2)) < epsilon22 * ab4)
        return true;
    }
    ao = bo;
  }
  return false;
}
function containsPolygon(coordinates2, point) {
  return !!polygonContains_default(coordinates2.map(ringRadians), pointRadians(point));
}
function ringRadians(ring) {
  return ring = ring.map(pointRadians), ring.pop(), ring;
}
function pointRadians(point) {
  return [point[0] * radians2, point[1] * radians2];
}
function contains_default2(object2, point) {
  return (object2 && containsObjectType.hasOwnProperty(object2.type) ? containsObjectType[object2.type] : containsGeometry)(object2, point);
}
// node_modules/d3-geo/src/graticule.js
function graticuleX(y02, y1, dy) {
  var y3 = range(y02, y1 - epsilon8, dy).concat(y1);
  return function(x3) {
    return y3.map(function(y4) {
      return [x3, y4];
    });
  };
}
function graticuleY(x02, x1, dx) {
  var x3 = range(x02, x1 - epsilon8, dx).concat(x1);
  return function(y3) {
    return x3.map(function(x4) {
      return [x4, y3];
    });
  };
}
function graticule() {
  var x1, x02, X12, X02, y1, y02, Y12, Y02, dx = 10, dy = dx, DX = 90, DY = 360, x3, y3, X3, Y3, precision = 2.5;
  function graticule2() {
    return { type: "MultiLineString", coordinates: lines() };
  }
  function lines() {
    return range(ceil(X02 / DX) * DX, X12, DX).map(X3).concat(range(ceil(Y02 / DY) * DY, Y12, DY).map(Y3)).concat(range(ceil(x02 / dx) * dx, x1, dx).filter(function(x4) {
      return abs3(x4 % DX) > epsilon8;
    }).map(x3)).concat(range(ceil(y02 / dy) * dy, y1, dy).filter(function(y4) {
      return abs3(y4 % DY) > epsilon8;
    }).map(y3));
  }
  graticule2.lines = function() {
    return lines().map(function(coordinates2) {
      return { type: "LineString", coordinates: coordinates2 };
    });
  };
  graticule2.outline = function() {
    return {
      type: "Polygon",
      coordinates: [
        X3(X02).concat(Y3(Y12).slice(1), X3(X12).reverse().slice(1), Y3(Y02).reverse().slice(1))
      ]
    };
  };
  graticule2.extent = function(_) {
    if (!arguments.length)
      return graticule2.extentMinor();
    return graticule2.extentMajor(_).extentMinor(_);
  };
  graticule2.extentMajor = function(_) {
    if (!arguments.length)
      return [[X02, Y02], [X12, Y12]];
    X02 = +_[0][0], X12 = +_[1][0];
    Y02 = +_[0][1], Y12 = +_[1][1];
    if (X02 > X12)
      _ = X02, X02 = X12, X12 = _;
    if (Y02 > Y12)
      _ = Y02, Y02 = Y12, Y12 = _;
    return graticule2.precision(precision);
  };
  graticule2.extentMinor = function(_) {
    if (!arguments.length)
      return [[x02, y02], [x1, y1]];
    x02 = +_[0][0], x1 = +_[1][0];
    y02 = +_[0][1], y1 = +_[1][1];
    if (x02 > x1)
      _ = x02, x02 = x1, x1 = _;
    if (y02 > y1)
      _ = y02, y02 = y1, y1 = _;
    return graticule2.precision(precision);
  };
  graticule2.step = function(_) {
    if (!arguments.length)
      return graticule2.stepMinor();
    return graticule2.stepMajor(_).stepMinor(_);
  };
  graticule2.stepMajor = function(_) {
    if (!arguments.length)
      return [DX, DY];
    DX = +_[0], DY = +_[1];
    return graticule2;
  };
  graticule2.stepMinor = function(_) {
    if (!arguments.length)
      return [dx, dy];
    dx = +_[0], dy = +_[1];
    return graticule2;
  };
  graticule2.precision = function(_) {
    if (!arguments.length)
      return precision;
    precision = +_;
    x3 = graticuleX(y02, y1, 90);
    y3 = graticuleY(x02, x1, precision);
    X3 = graticuleX(Y02, Y12, 90);
    Y3 = graticuleY(X02, X12, precision);
    return graticule2;
  };
  return graticule2.extentMajor([[-180, -90 + epsilon8], [180, 90 - epsilon8]]).extentMinor([[-180, -80 - epsilon8], [180, 80 + epsilon8]]);
}
function graticule10() {
  return graticule()();
}
// node_modules/d3-geo/src/interpolate.js
function interpolate_default2(a2, b) {
  var x02 = a2[0] * radians2, y02 = a2[1] * radians2, x1 = b[0] * radians2, y1 = b[1] * radians2, cy0 = cos2(y02), sy0 = sin2(y02), cy1 = cos2(y1), sy1 = sin2(y1), kx0 = cy0 * cos2(x02), ky0 = cy0 * sin2(x02), kx1 = cy1 * cos2(x1), ky1 = cy1 * sin2(x1), d = 2 * asin(sqrt(haversin(y1 - y02) + cy0 * cy1 * haversin(x1 - x02))), k = sin2(d);
  var interpolate = d ? function(t) {
    var B3 = sin2(t *= d) / k, A2 = sin2(d - t) / k, x3 = A2 * kx0 + B3 * kx1, y3 = A2 * ky0 + B3 * ky1, z = A2 * sy0 + B3 * sy1;
    return [
      atan2(y3, x3) * degrees3,
      atan2(z, sqrt(x3 * x3 + y3 * y3)) * degrees3
    ];
  } : function() {
    return [x02 * degrees3, y02 * degrees3];
  };
  interpolate.distance = d;
  return interpolate;
}
// node_modules/d3-geo/src/identity.js
var identity_default3 = (x3) => x3;

// node_modules/d3-geo/src/path/area.js
var areaSum2 = new Adder;
var areaRingSum2 = new Adder;
var x00;
var y00;
var x02;
var y02;
var areaStream2 = {
  point: noop2,
  lineStart: noop2,
  lineEnd: noop2,
  polygonStart: function() {
    areaStream2.lineStart = areaRingStart2;
    areaStream2.lineEnd = areaRingEnd2;
  },
  polygonEnd: function() {
    areaStream2.lineStart = areaStream2.lineEnd = areaStream2.point = noop2;
    areaSum2.add(abs3(areaRingSum2));
    areaRingSum2 = new Adder;
  },
  result: function() {
    var area = areaSum2 / 2;
    areaSum2 = new Adder;
    return area;
  }
};
function areaRingStart2() {
  areaStream2.point = areaPointFirst2;
}
function areaPointFirst2(x3, y3) {
  areaStream2.point = areaPoint2;
  x00 = x02 = x3, y00 = y02 = y3;
}
function areaPoint2(x3, y3) {
  areaRingSum2.add(y02 * x3 - x02 * y3);
  x02 = x3, y02 = y3;
}
function areaRingEnd2() {
  areaPoint2(x00, y00);
}
var area_default3 = areaStream2;

// node_modules/d3-geo/src/path/bounds.js
var x03 = Infinity;
var y03 = x03;
var x1 = -x03;
var y1 = x1;
var boundsStream2 = {
  point: boundsPoint2,
  lineStart: noop2,
  lineEnd: noop2,
  polygonStart: noop2,
  polygonEnd: noop2,
  result: function() {
    var bounds = [[x03, y03], [x1, y1]];
    x1 = y1 = -(y03 = x03 = Infinity);
    return bounds;
  }
};
function boundsPoint2(x3, y3) {
  if (x3 < x03)
    x03 = x3;
  if (x3 > x1)
    x1 = x3;
  if (y3 < y03)
    y03 = y3;
  if (y3 > y1)
    y1 = y3;
}
var bounds_default2 = boundsStream2;

// node_modules/d3-geo/src/path/centroid.js
var X02 = 0;
var Y02 = 0;
var Z02 = 0;
var X12 = 0;
var Y12 = 0;
var Z12 = 0;
var X22 = 0;
var Y22 = 0;
var Z22 = 0;
var x002;
var y002;
var x04;
var y04;
var centroidStream2 = {
  point: centroidPoint2,
  lineStart: centroidLineStart2,
  lineEnd: centroidLineEnd2,
  polygonStart: function() {
    centroidStream2.lineStart = centroidRingStart2;
    centroidStream2.lineEnd = centroidRingEnd2;
  },
  polygonEnd: function() {
    centroidStream2.point = centroidPoint2;
    centroidStream2.lineStart = centroidLineStart2;
    centroidStream2.lineEnd = centroidLineEnd2;
  },
  result: function() {
    var centroid = Z22 ? [X22 / Z22, Y22 / Z22] : Z12 ? [X12 / Z12, Y12 / Z12] : Z02 ? [X02 / Z02, Y02 / Z02] : [NaN, NaN];
    X02 = Y02 = Z02 = X12 = Y12 = Z12 = X22 = Y22 = Z22 = 0;
    return centroid;
  }
};
function centroidPoint2(x3, y3) {
  X02 += x3;
  Y02 += y3;
  ++Z02;
}
function centroidLineStart2() {
  centroidStream2.point = centroidPointFirstLine;
}
function centroidPointFirstLine(x3, y3) {
  centroidStream2.point = centroidPointLine;
  centroidPoint2(x04 = x3, y04 = y3);
}
function centroidPointLine(x3, y3) {
  var dx = x3 - x04, dy = y3 - y04, z = sqrt(dx * dx + dy * dy);
  X12 += z * (x04 + x3) / 2;
  Y12 += z * (y04 + y3) / 2;
  Z12 += z;
  centroidPoint2(x04 = x3, y04 = y3);
}
function centroidLineEnd2() {
  centroidStream2.point = centroidPoint2;
}
function centroidRingStart2() {
  centroidStream2.point = centroidPointFirstRing;
}
function centroidRingEnd2() {
  centroidPointRing(x002, y002);
}
function centroidPointFirstRing(x3, y3) {
  centroidStream2.point = centroidPointRing;
  centroidPoint2(x002 = x04 = x3, y002 = y04 = y3);
}
function centroidPointRing(x3, y3) {
  var dx = x3 - x04, dy = y3 - y04, z = sqrt(dx * dx + dy * dy);
  X12 += z * (x04 + x3) / 2;
  Y12 += z * (y04 + y3) / 2;
  Z12 += z;
  z = y04 * x3 - x04 * y3;
  X22 += z * (x04 + x3);
  Y22 += z * (y04 + y3);
  Z22 += z * 3;
  centroidPoint2(x04 = x3, y04 = y3);
}
var centroid_default2 = centroidStream2;

// node_modules/d3-geo/src/path/context.js
function PathContext(context) {
  this._context = context;
}
PathContext.prototype = {
  _radius: 4.5,
  pointRadius: function(_) {
    return this._radius = _, this;
  },
  polygonStart: function() {
    this._line = 0;
  },
  polygonEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._point = 0;
  },
  lineEnd: function() {
    if (this._line === 0)
      this._context.closePath();
    this._point = NaN;
  },
  point: function(x3, y3) {
    switch (this._point) {
      case 0: {
        this._context.moveTo(x3, y3);
        this._point = 1;
        break;
      }
      case 1: {
        this._context.lineTo(x3, y3);
        break;
      }
      default: {
        this._context.moveTo(x3 + this._radius, y3);
        this._context.arc(x3, y3, this._radius, 0, tau5);
        break;
      }
    }
  },
  result: noop2
};

// node_modules/d3-geo/src/path/measure.js
var lengthSum2 = new Adder;
var lengthRing;
var x003;
var y003;
var x05;
var y05;
var lengthStream2 = {
  point: noop2,
  lineStart: function() {
    lengthStream2.point = lengthPointFirst2;
  },
  lineEnd: function() {
    if (lengthRing)
      lengthPoint2(x003, y003);
    lengthStream2.point = noop2;
  },
  polygonStart: function() {
    lengthRing = true;
  },
  polygonEnd: function() {
    lengthRing = null;
  },
  result: function() {
    var length3 = +lengthSum2;
    lengthSum2 = new Adder;
    return length3;
  }
};
function lengthPointFirst2(x3, y3) {
  lengthStream2.point = lengthPoint2;
  x003 = x05 = x3, y003 = y05 = y3;
}
function lengthPoint2(x3, y3) {
  x05 -= x3, y05 -= y3;
  lengthSum2.add(sqrt(x05 * x05 + y05 * y05));
  x05 = x3, y05 = y3;
}
var measure_default = lengthStream2;

// node_modules/d3-geo/src/path/string.js
var cacheDigits;
var cacheAppend;
var cacheRadius;
var cacheCircle;

class PathString {
  constructor(digits) {
    this._append = digits == null ? append2 : appendRound2(digits);
    this._radius = 4.5;
    this._ = "";
  }
  pointRadius(_) {
    this._radius = +_;
    return this;
  }
  polygonStart() {
    this._line = 0;
  }
  polygonEnd() {
    this._line = NaN;
  }
  lineStart() {
    this._point = 0;
  }
  lineEnd() {
    if (this._line === 0)
      this._ += "Z";
    this._point = NaN;
  }
  point(x3, y3) {
    switch (this._point) {
      case 0: {
        this._append`M${x3},${y3}`;
        this._point = 1;
        break;
      }
      case 1: {
        this._append`L${x3},${y3}`;
        break;
      }
      default: {
        this._append`M${x3},${y3}`;
        if (this._radius !== cacheRadius || this._append !== cacheAppend) {
          const r = this._radius;
          const s = this._;
          this._ = "";
          this._append`m0,${r}a${r},${r} 0 1,1 0,${-2 * r}a${r},${r} 0 1,1 0,${2 * r}z`;
          cacheRadius = r;
          cacheAppend = this._append;
          cacheCircle = this._;
          this._ = s;
        }
        this._ += cacheCircle;
        break;
      }
    }
  }
  result() {
    const result = this._;
    this._ = "";
    return result.length ? result : null;
  }
}
function append2(strings) {
  let i = 1;
  this._ += strings[0];
  for (const j = strings.length;i < j; ++i) {
    this._ += arguments[i] + strings[i];
  }
}
function appendRound2(digits) {
  const d = Math.floor(digits);
  if (!(d >= 0))
    throw new RangeError(`invalid digits: ${digits}`);
  if (d > 15)
    return append2;
  if (d !== cacheDigits) {
    const k = 10 ** d;
    cacheDigits = d;
    cacheAppend = function append3(strings) {
      let i = 1;
      this._ += strings[0];
      for (const j = strings.length;i < j; ++i) {
        this._ += Math.round(arguments[i] * k) / k + strings[i];
      }
    };
  }
  return cacheAppend;
}

// node_modules/d3-geo/src/path/index.js
function path_default(projection, context) {
  let digits = 3, pointRadius = 4.5, projectionStream, contextStream;
  function path2(object2) {
    if (object2) {
      if (typeof pointRadius === "function")
        contextStream.pointRadius(+pointRadius.apply(this, arguments));
      stream_default(object2, projectionStream(contextStream));
    }
    return contextStream.result();
  }
  path2.area = function(object2) {
    stream_default(object2, projectionStream(area_default3));
    return area_default3.result();
  };
  path2.measure = function(object2) {
    stream_default(object2, projectionStream(measure_default));
    return measure_default.result();
  };
  path2.bounds = function(object2) {
    stream_default(object2, projectionStream(bounds_default2));
    return bounds_default2.result();
  };
  path2.centroid = function(object2) {
    stream_default(object2, projectionStream(centroid_default2));
    return centroid_default2.result();
  };
  path2.projection = function(_) {
    if (!arguments.length)
      return projection;
    projectionStream = _ == null ? (projection = null, identity_default3) : (projection = _).stream;
    return path2;
  };
  path2.context = function(_) {
    if (!arguments.length)
      return context;
    contextStream = _ == null ? (context = null, new PathString(digits)) : new PathContext(context = _);
    if (typeof pointRadius !== "function")
      contextStream.pointRadius(pointRadius);
    return path2;
  };
  path2.pointRadius = function(_) {
    if (!arguments.length)
      return pointRadius;
    pointRadius = typeof _ === "function" ? _ : (contextStream.pointRadius(+_), +_);
    return path2;
  };
  path2.digits = function(_) {
    if (!arguments.length)
      return digits;
    if (_ == null)
      digits = null;
    else {
      const d = Math.floor(_);
      if (!(d >= 0))
        throw new RangeError(`invalid digits: ${_}`);
      digits = d;
    }
    if (context === null)
      contextStream = new PathString(digits);
    return path2;
  };
  return path2.projection(projection).digits(digits).context(context);
}
// node_modules/d3-geo/src/transform.js
function transform_default(methods) {
  return {
    stream: transformer(methods)
  };
}
function transformer(methods) {
  return function(stream) {
    var s = new TransformStream;
    for (var key in methods)
      s[key] = methods[key];
    s.stream = stream;
    return s;
  };
}
function TransformStream() {}
TransformStream.prototype = {
  constructor: TransformStream,
  point: function(x3, y3) {
    this.stream.point(x3, y3);
  },
  sphere: function() {
    this.stream.sphere();
  },
  lineStart: function() {
    this.stream.lineStart();
  },
  lineEnd: function() {
    this.stream.lineEnd();
  },
  polygonStart: function() {
    this.stream.polygonStart();
  },
  polygonEnd: function() {
    this.stream.polygonEnd();
  }
};

// node_modules/d3-geo/src/projection/fit.js
function fit(projection, fitBounds, object2) {
  var clip = projection.clipExtent && projection.clipExtent();
  projection.scale(150).translate([0, 0]);
  if (clip != null)
    projection.clipExtent(null);
  stream_default(object2, projection.stream(bounds_default2));
  fitBounds(bounds_default2.result());
  if (clip != null)
    projection.clipExtent(clip);
  return projection;
}
function fitExtent(projection, extent2, object2) {
  return fit(projection, function(b) {
    var w = extent2[1][0] - extent2[0][0], h = extent2[1][1] - extent2[0][1], k = Math.min(w / (b[1][0] - b[0][0]), h / (b[1][1] - b[0][1])), x3 = +extent2[0][0] + (w - k * (b[1][0] + b[0][0])) / 2, y3 = +extent2[0][1] + (h - k * (b[1][1] + b[0][1])) / 2;
    projection.scale(150 * k).translate([x3, y3]);
  }, object2);
}
function fitSize(projection, size, object2) {
  return fitExtent(projection, [[0, 0], size], object2);
}
function fitWidth(projection, width, object2) {
  return fit(projection, function(b) {
    var w = +width, k = w / (b[1][0] - b[0][0]), x3 = (w - k * (b[1][0] + b[0][0])) / 2, y3 = -k * b[0][1];
    projection.scale(150 * k).translate([x3, y3]);
  }, object2);
}
function fitHeight(projection, height, object2) {
  return fit(projection, function(b) {
    var h = +height, k = h / (b[1][1] - b[0][1]), x3 = -k * b[0][0], y3 = (h - k * (b[1][1] + b[0][1])) / 2;
    projection.scale(150 * k).translate([x3, y3]);
  }, object2);
}

// node_modules/d3-geo/src/projection/resample.js
var maxDepth = 16;
var cosMinDistance = cos2(30 * radians2);
function resample_default(project, delta2) {
  return +delta2 ? resample(project, delta2) : resampleNone(project);
}
function resampleNone(project) {
  return transformer({
    point: function(x3, y3) {
      x3 = project(x3, y3);
      this.stream.point(x3[0], x3[1]);
    }
  });
}
function resample(project, delta2) {
  function resampleLineTo(x06, y06, lambda04, a0, b02, c0, x12, y12, lambda12, a1, b12, c1, depth, stream) {
    var dx = x12 - x06, dy = y12 - y06, d2 = dx * dx + dy * dy;
    if (d2 > 4 * delta2 && depth--) {
      var a2 = a0 + a1, b = b02 + b12, c2 = c0 + c1, m2 = sqrt(a2 * a2 + b * b + c2 * c2), phi2 = asin(c2 /= m2), lambda22 = abs3(abs3(c2) - 1) < epsilon8 || abs3(lambda04 - lambda12) < epsilon8 ? (lambda04 + lambda12) / 2 : atan2(b, a2), p = project(lambda22, phi2), x22 = p[0], y22 = p[1], dx2 = x22 - x06, dy2 = y22 - y06, dz = dy * dx2 - dx * dy2;
      if (dz * dz / d2 > delta2 || abs3((dx * dx2 + dy * dy2) / d2 - 0.5) > 0.3 || a0 * a1 + b02 * b12 + c0 * c1 < cosMinDistance) {
        resampleLineTo(x06, y06, lambda04, a0, b02, c0, x22, y22, lambda22, a2 /= m2, b /= m2, c2, depth, stream);
        stream.point(x22, y22);
        resampleLineTo(x22, y22, lambda22, a2, b, c2, x12, y12, lambda12, a1, b12, c1, depth, stream);
      }
    }
  }
  return function(stream) {
    var lambda004, x004, y004, a00, b00, c00, lambda04, x06, y06, a0, b02, c0;
    var resampleStream = {
      point,
      lineStart,
      lineEnd,
      polygonStart: function() {
        stream.polygonStart();
        resampleStream.lineStart = ringStart;
      },
      polygonEnd: function() {
        stream.polygonEnd();
        resampleStream.lineStart = lineStart;
      }
    };
    function point(x3, y3) {
      x3 = project(x3, y3);
      stream.point(x3[0], x3[1]);
    }
    function lineStart() {
      x06 = NaN;
      resampleStream.point = linePoint2;
      stream.lineStart();
    }
    function linePoint2(lambda, phi) {
      var c2 = cartesian([lambda, phi]), p = project(lambda, phi);
      resampleLineTo(x06, y06, lambda04, a0, b02, c0, x06 = p[0], y06 = p[1], lambda04 = lambda, a0 = c2[0], b02 = c2[1], c0 = c2[2], maxDepth, stream);
      stream.point(x06, y06);
    }
    function lineEnd() {
      resampleStream.point = point;
      stream.lineEnd();
    }
    function ringStart() {
      lineStart();
      resampleStream.point = ringPoint;
      resampleStream.lineEnd = ringEnd;
    }
    function ringPoint(lambda, phi) {
      linePoint2(lambda004 = lambda, phi), x004 = x06, y004 = y06, a00 = a0, b00 = b02, c00 = c0;
      resampleStream.point = linePoint2;
    }
    function ringEnd() {
      resampleLineTo(x06, y06, lambda04, a0, b02, c0, x004, y004, lambda004, a00, b00, c00, maxDepth, stream);
      resampleStream.lineEnd = lineEnd;
      lineEnd();
    }
    return resampleStream;
  };
}

// node_modules/d3-geo/src/projection/index.js
var transformRadians = transformer({
  point: function(x3, y3) {
    this.stream.point(x3 * radians2, y3 * radians2);
  }
});
function transformRotate(rotate) {
  return transformer({
    point: function(x3, y3) {
      var r = rotate(x3, y3);
      return this.stream.point(r[0], r[1]);
    }
  });
}
function scaleTranslate(k, dx, dy, sx, sy) {
  function transform(x3, y3) {
    x3 *= sx;
    y3 *= sy;
    return [dx + k * x3, dy - k * y3];
  }
  transform.invert = function(x3, y3) {
    return [(x3 - dx) / k * sx, (dy - y3) / k * sy];
  };
  return transform;
}
function scaleTranslateRotate(k, dx, dy, sx, sy, alpha) {
  if (!alpha)
    return scaleTranslate(k, dx, dy, sx, sy);
  var cosAlpha = cos2(alpha), sinAlpha = sin2(alpha), a2 = cosAlpha * k, b = sinAlpha * k, ai = cosAlpha / k, bi = sinAlpha / k, ci = (sinAlpha * dy - cosAlpha * dx) / k, fi = (sinAlpha * dx + cosAlpha * dy) / k;
  function transform(x3, y3) {
    x3 *= sx;
    y3 *= sy;
    return [a2 * x3 - b * y3 + dx, dy - b * x3 - a2 * y3];
  }
  transform.invert = function(x3, y3) {
    return [sx * (ai * x3 - bi * y3 + ci), sy * (fi - bi * x3 - ai * y3)];
  };
  return transform;
}
function projection(project) {
  return projectionMutator(function() {
    return project;
  })();
}
function projectionMutator(projectAt) {
  var project, k = 150, x3 = 480, y3 = 250, lambda = 0, phi = 0, deltaLambda = 0, deltaPhi = 0, deltaGamma = 0, rotate, alpha = 0, sx = 1, sy = 1, theta = null, preclip = antimeridian_default, x06 = null, y06, x12, y12, postclip = identity_default3, delta2 = 0.5, projectResample, projectTransform, projectRotateTransform, cache, cacheStream;
  function projection2(point) {
    return projectRotateTransform(point[0] * radians2, point[1] * radians2);
  }
  function invert(point) {
    point = projectRotateTransform.invert(point[0], point[1]);
    return point && [point[0] * degrees3, point[1] * degrees3];
  }
  projection2.stream = function(stream) {
    return cache && cacheStream === stream ? cache : cache = transformRadians(transformRotate(rotate)(preclip(projectResample(postclip(cacheStream = stream)))));
  };
  projection2.preclip = function(_) {
    return arguments.length ? (preclip = _, theta = undefined, reset()) : preclip;
  };
  projection2.postclip = function(_) {
    return arguments.length ? (postclip = _, x06 = y06 = x12 = y12 = null, reset()) : postclip;
  };
  projection2.clipAngle = function(_) {
    return arguments.length ? (preclip = +_ ? circle_default2(theta = _ * radians2) : (theta = null, antimeridian_default), reset()) : theta * degrees3;
  };
  projection2.clipExtent = function(_) {
    return arguments.length ? (postclip = _ == null ? (x06 = y06 = x12 = y12 = null, identity_default3) : clipRectangle(x06 = +_[0][0], y06 = +_[0][1], x12 = +_[1][0], y12 = +_[1][1]), reset()) : x06 == null ? null : [[x06, y06], [x12, y12]];
  };
  projection2.scale = function(_) {
    return arguments.length ? (k = +_, recenter()) : k;
  };
  projection2.translate = function(_) {
    return arguments.length ? (x3 = +_[0], y3 = +_[1], recenter()) : [x3, y3];
  };
  projection2.center = function(_) {
    return arguments.length ? (lambda = _[0] % 360 * radians2, phi = _[1] % 360 * radians2, recenter()) : [lambda * degrees3, phi * degrees3];
  };
  projection2.rotate = function(_) {
    return arguments.length ? (deltaLambda = _[0] % 360 * radians2, deltaPhi = _[1] % 360 * radians2, deltaGamma = _.length > 2 ? _[2] % 360 * radians2 : 0, recenter()) : [deltaLambda * degrees3, deltaPhi * degrees3, deltaGamma * degrees3];
  };
  projection2.angle = function(_) {
    return arguments.length ? (alpha = _ % 360 * radians2, recenter()) : alpha * degrees3;
  };
  projection2.reflectX = function(_) {
    return arguments.length ? (sx = _ ? -1 : 1, recenter()) : sx < 0;
  };
  projection2.reflectY = function(_) {
    return arguments.length ? (sy = _ ? -1 : 1, recenter()) : sy < 0;
  };
  projection2.precision = function(_) {
    return arguments.length ? (projectResample = resample_default(projectTransform, delta2 = _ * _), reset()) : sqrt(delta2);
  };
  projection2.fitExtent = function(extent2, object2) {
    return fitExtent(projection2, extent2, object2);
  };
  projection2.fitSize = function(size, object2) {
    return fitSize(projection2, size, object2);
  };
  projection2.fitWidth = function(width, object2) {
    return fitWidth(projection2, width, object2);
  };
  projection2.fitHeight = function(height, object2) {
    return fitHeight(projection2, height, object2);
  };
  function recenter() {
    var center2 = scaleTranslateRotate(k, 0, 0, sx, sy, alpha).apply(null, project(lambda, phi)), transform = scaleTranslateRotate(k, x3 - center2[0], y3 - center2[1], sx, sy, alpha);
    rotate = rotateRadians(deltaLambda, deltaPhi, deltaGamma);
    projectTransform = compose_default(project, transform);
    projectRotateTransform = compose_default(rotate, projectTransform);
    projectResample = resample_default(projectTransform, delta2);
    return reset();
  }
  function reset() {
    cache = cacheStream = null;
    return projection2;
  }
  return function() {
    project = projectAt.apply(this, arguments);
    projection2.invert = project.invert && invert;
    return recenter();
  };
}

// node_modules/d3-geo/src/projection/conic.js
function conicProjection(projectAt) {
  var phi02 = 0, phi12 = pi4 / 3, m2 = projectionMutator(projectAt), p = m2(phi02, phi12);
  p.parallels = function(_) {
    return arguments.length ? m2(phi02 = _[0] * radians2, phi12 = _[1] * radians2) : [phi02 * degrees3, phi12 * degrees3];
  };
  return p;
}

// node_modules/d3-geo/src/projection/cylindricalEqualArea.js
function cylindricalEqualAreaRaw(phi02) {
  var cosPhi03 = cos2(phi02);
  function forward(lambda, phi) {
    return [lambda * cosPhi03, sin2(phi) / cosPhi03];
  }
  forward.invert = function(x3, y3) {
    return [x3 / cosPhi03, asin(y3 * cosPhi03)];
  };
  return forward;
}

// node_modules/d3-geo/src/projection/conicEqualArea.js
function conicEqualAreaRaw(y06, y12) {
  var sy0 = sin2(y06), n = (sy0 + sin2(y12)) / 2;
  if (abs3(n) < epsilon8)
    return cylindricalEqualAreaRaw(y06);
  var c2 = 1 + sy0 * (2 * n - sy0), r0 = sqrt(c2) / n;
  function project(x3, y3) {
    var r = sqrt(c2 - 2 * n * sin2(y3)) / n;
    return [r * sin2(x3 *= n), r0 - r * cos2(x3)];
  }
  project.invert = function(x3, y3) {
    var r0y = r0 - y3, l = atan2(x3, abs3(r0y)) * sign(r0y);
    if (r0y * n < 0)
      l -= pi4 * sign(x3) * sign(r0y);
    return [l / n, asin((c2 - (x3 * x3 + r0y * r0y) * n * n) / (2 * n))];
  };
  return project;
}
function conicEqualArea_default() {
  return conicProjection(conicEqualAreaRaw).scale(155.424).center([0, 33.6442]);
}

// node_modules/d3-geo/src/projection/albers.js
function albers_default() {
  return conicEqualArea_default().parallels([29.5, 45.5]).scale(1070).translate([480, 250]).rotate([96, 0]).center([-0.6, 38.7]);
}
// node_modules/d3-geo/src/projection/albersUsa.js
function multiplex(streams) {
  var n = streams.length;
  return {
    point: function(x3, y3) {
      var i = -1;
      while (++i < n)
        streams[i].point(x3, y3);
    },
    sphere: function() {
      var i = -1;
      while (++i < n)
        streams[i].sphere();
    },
    lineStart: function() {
      var i = -1;
      while (++i < n)
        streams[i].lineStart();
    },
    lineEnd: function() {
      var i = -1;
      while (++i < n)
        streams[i].lineEnd();
    },
    polygonStart: function() {
      var i = -1;
      while (++i < n)
        streams[i].polygonStart();
    },
    polygonEnd: function() {
      var i = -1;
      while (++i < n)
        streams[i].polygonEnd();
    }
  };
}
function albersUsa_default() {
  var cache, cacheStream, lower48 = albers_default(), lower48Point, alaska = conicEqualArea_default().rotate([154, 0]).center([-2, 58.5]).parallels([55, 65]), alaskaPoint, hawaii = conicEqualArea_default().rotate([157, 0]).center([-3, 19.9]).parallels([8, 18]), hawaiiPoint, point, pointStream = { point: function(x3, y3) {
    point = [x3, y3];
  } };
  function albersUsa(coordinates2) {
    var x3 = coordinates2[0], y3 = coordinates2[1];
    return point = null, (lower48Point.point(x3, y3), point) || (alaskaPoint.point(x3, y3), point) || (hawaiiPoint.point(x3, y3), point);
  }
  albersUsa.invert = function(coordinates2) {
    var k = lower48.scale(), t = lower48.translate(), x3 = (coordinates2[0] - t[0]) / k, y3 = (coordinates2[1] - t[1]) / k;
    return (y3 >= 0.12 && y3 < 0.234 && x3 >= -0.425 && x3 < -0.214 ? alaska : y3 >= 0.166 && y3 < 0.234 && x3 >= -0.214 && x3 < -0.115 ? hawaii : lower48).invert(coordinates2);
  };
  albersUsa.stream = function(stream) {
    return cache && cacheStream === stream ? cache : cache = multiplex([lower48.stream(cacheStream = stream), alaska.stream(stream), hawaii.stream(stream)]);
  };
  albersUsa.precision = function(_) {
    if (!arguments.length)
      return lower48.precision();
    lower48.precision(_), alaska.precision(_), hawaii.precision(_);
    return reset();
  };
  albersUsa.scale = function(_) {
    if (!arguments.length)
      return lower48.scale();
    lower48.scale(_), alaska.scale(_ * 0.35), hawaii.scale(_);
    return albersUsa.translate(lower48.translate());
  };
  albersUsa.translate = function(_) {
    if (!arguments.length)
      return lower48.translate();
    var k = lower48.scale(), x3 = +_[0], y3 = +_[1];
    lower48Point = lower48.translate(_).clipExtent([[x3 - 0.455 * k, y3 - 0.238 * k], [x3 + 0.455 * k, y3 + 0.238 * k]]).stream(pointStream);
    alaskaPoint = alaska.translate([x3 - 0.307 * k, y3 + 0.201 * k]).clipExtent([[x3 - 0.425 * k + epsilon8, y3 + 0.12 * k + epsilon8], [x3 - 0.214 * k - epsilon8, y3 + 0.234 * k - epsilon8]]).stream(pointStream);
    hawaiiPoint = hawaii.translate([x3 - 0.205 * k, y3 + 0.212 * k]).clipExtent([[x3 - 0.214 * k + epsilon8, y3 + 0.166 * k + epsilon8], [x3 - 0.115 * k - epsilon8, y3 + 0.234 * k - epsilon8]]).stream(pointStream);
    return reset();
  };
  albersUsa.fitExtent = function(extent2, object2) {
    return fitExtent(albersUsa, extent2, object2);
  };
  albersUsa.fitSize = function(size, object2) {
    return fitSize(albersUsa, size, object2);
  };
  albersUsa.fitWidth = function(width, object2) {
    return fitWidth(albersUsa, width, object2);
  };
  albersUsa.fitHeight = function(height, object2) {
    return fitHeight(albersUsa, height, object2);
  };
  function reset() {
    cache = cacheStream = null;
    return albersUsa;
  }
  return albersUsa.scale(1070);
}
// node_modules/d3-geo/src/projection/azimuthal.js
function azimuthalRaw(scale2) {
  return function(x3, y3) {
    var cx = cos2(x3), cy = cos2(y3), k = scale2(cx * cy);
    if (k === Infinity)
      return [2, 0];
    return [
      k * cy * sin2(x3),
      k * sin2(y3)
    ];
  };
}
function azimuthalInvert(angle2) {
  return function(x3, y3) {
    var z = sqrt(x3 * x3 + y3 * y3), c2 = angle2(z), sc = sin2(c2), cc2 = cos2(c2);
    return [
      atan2(x3 * sc, z * cc2),
      asin(z && y3 * sc / z)
    ];
  };
}

// node_modules/d3-geo/src/projection/azimuthalEqualArea.js
var azimuthalEqualAreaRaw = azimuthalRaw(function(cxcy) {
  return sqrt(2 / (1 + cxcy));
});
azimuthalEqualAreaRaw.invert = azimuthalInvert(function(z) {
  return 2 * asin(z / 2);
});
function azimuthalEqualArea_default() {
  return projection(azimuthalEqualAreaRaw).scale(124.75).clipAngle(180 - 0.001);
}
// node_modules/d3-geo/src/projection/azimuthalEquidistant.js
var azimuthalEquidistantRaw = azimuthalRaw(function(c2) {
  return (c2 = acos(c2)) && c2 / sin2(c2);
});
azimuthalEquidistantRaw.invert = azimuthalInvert(function(z) {
  return z;
});
function azimuthalEquidistant_default() {
  return projection(azimuthalEquidistantRaw).scale(79.4188).clipAngle(180 - 0.001);
}
// node_modules/d3-geo/src/projection/mercator.js
function mercatorRaw(lambda, phi) {
  return [lambda, log(tan((halfPi3 + phi) / 2))];
}
mercatorRaw.invert = function(x3, y3) {
  return [x3, 2 * atan(exp(y3)) - halfPi3];
};
function mercator_default() {
  return mercatorProjection(mercatorRaw).scale(961 / tau5);
}
function mercatorProjection(project) {
  var m2 = projection(project), center2 = m2.center, scale2 = m2.scale, translate = m2.translate, clipExtent = m2.clipExtent, x06 = null, y06, x12, y12;
  m2.scale = function(_) {
    return arguments.length ? (scale2(_), reclip()) : scale2();
  };
  m2.translate = function(_) {
    return arguments.length ? (translate(_), reclip()) : translate();
  };
  m2.center = function(_) {
    return arguments.length ? (center2(_), reclip()) : center2();
  };
  m2.clipExtent = function(_) {
    return arguments.length ? (_ == null ? x06 = y06 = x12 = y12 = null : (x06 = +_[0][0], y06 = +_[0][1], x12 = +_[1][0], y12 = +_[1][1]), reclip()) : x06 == null ? null : [[x06, y06], [x12, y12]];
  };
  function reclip() {
    var k = pi4 * scale2(), t = m2(rotation_default(m2.rotate()).invert([0, 0]));
    return clipExtent(x06 == null ? [[t[0] - k, t[1] - k], [t[0] + k, t[1] + k]] : project === mercatorRaw ? [[Math.max(t[0] - k, x06), y06], [Math.min(t[0] + k, x12), y12]] : [[x06, Math.max(t[1] - k, y06)], [x12, Math.min(t[1] + k, y12)]]);
  }
  return reclip();
}

// node_modules/d3-geo/src/projection/conicConformal.js
function tany(y3) {
  return tan((halfPi3 + y3) / 2);
}
function conicConformalRaw(y06, y12) {
  var cy0 = cos2(y06), n = y06 === y12 ? sin2(y06) : log(cy0 / cos2(y12)) / log(tany(y12) / tany(y06)), f = cy0 * pow2(tany(y06), n) / n;
  if (!n)
    return mercatorRaw;
  function project(x3, y3) {
    if (f > 0) {
      if (y3 < -halfPi3 + epsilon8)
        y3 = -halfPi3 + epsilon8;
    } else {
      if (y3 > halfPi3 - epsilon8)
        y3 = halfPi3 - epsilon8;
    }
    var r = f / pow2(tany(y3), n);
    return [r * sin2(n * x3), f - r * cos2(n * x3)];
  }
  project.invert = function(x3, y3) {
    var fy = f - y3, r = sign(n) * sqrt(x3 * x3 + fy * fy), l = atan2(x3, abs3(fy)) * sign(fy);
    if (fy * n < 0)
      l -= pi4 * sign(x3) * sign(fy);
    return [l / n, 2 * atan(pow2(f / r, 1 / n)) - halfPi3];
  };
  return project;
}
function conicConformal_default() {
  return conicProjection(conicConformalRaw).scale(109.5).parallels([30, 30]);
}
// node_modules/d3-geo/src/projection/equirectangular.js
function equirectangularRaw(lambda, phi) {
  return [lambda, phi];
}
equirectangularRaw.invert = equirectangularRaw;
function equirectangular_default() {
  return projection(equirectangularRaw).scale(152.63);
}

// node_modules/d3-geo/src/projection/conicEquidistant.js
function conicEquidistantRaw(y06, y12) {
  var cy0 = cos2(y06), n = y06 === y12 ? sin2(y06) : (cy0 - cos2(y12)) / (y12 - y06), g = cy0 / n + y06;
  if (abs3(n) < epsilon8)
    return equirectangularRaw;
  function project(x3, y3) {
    var gy = g - y3, nx = n * x3;
    return [gy * sin2(nx), g - gy * cos2(nx)];
  }
  project.invert = function(x3, y3) {
    var gy = g - y3, l = atan2(x3, abs3(gy)) * sign(gy);
    if (gy * n < 0)
      l -= pi4 * sign(x3) * sign(gy);
    return [l / n, g - sign(n) * sqrt(x3 * x3 + gy * gy)];
  };
  return project;
}
function conicEquidistant_default() {
  return conicProjection(conicEquidistantRaw).scale(131.154).center([0, 13.9389]);
}
// node_modules/d3-geo/src/projection/equalEarth.js
var A1 = 1.340264;
var A2 = -0.081106;
var A3 = 0.000893;
var A4 = 0.003796;
var M = sqrt(3) / 2;
var iterations = 12;
function equalEarthRaw(lambda, phi) {
  var l = asin(M * sin2(phi)), l2 = l * l, l6 = l2 * l2 * l2;
  return [
    lambda * cos2(l) / (M * (A1 + 3 * A2 * l2 + l6 * (7 * A3 + 9 * A4 * l2))),
    l * (A1 + A2 * l2 + l6 * (A3 + A4 * l2))
  ];
}
equalEarthRaw.invert = function(x3, y3) {
  var l = y3, l2 = l * l, l6 = l2 * l2 * l2;
  for (var i = 0, delta, fy, fpy;i < iterations; ++i) {
    fy = l * (A1 + A2 * l2 + l6 * (A3 + A4 * l2)) - y3;
    fpy = A1 + 3 * A2 * l2 + l6 * (7 * A3 + 9 * A4 * l2);
    l -= delta = fy / fpy, l2 = l * l, l6 = l2 * l2 * l2;
    if (abs3(delta) < epsilon22)
      break;
  }
  return [
    M * x3 * (A1 + 3 * A2 * l2 + l6 * (7 * A3 + 9 * A4 * l2)) / cos2(l),
    asin(sin2(l) / M)
  ];
};
function equalEarth_default() {
  return projection(equalEarthRaw).scale(177.158);
}
// node_modules/d3-geo/src/projection/gnomonic.js
function gnomonicRaw(x3, y3) {
  var cy = cos2(y3), k = cos2(x3) * cy;
  return [cy * sin2(x3) / k, sin2(y3) / k];
}
gnomonicRaw.invert = azimuthalInvert(atan);
function gnomonic_default() {
  return projection(gnomonicRaw).scale(144.049).clipAngle(60);
}
// node_modules/d3-geo/src/projection/identity.js
function identity_default4() {
  var k = 1, tx = 0, ty = 0, sx = 1, sy = 1, alpha = 0, ca3, sa, x06 = null, y06, x12, y12, kx = 1, ky = 1, transform = transformer({
    point: function(x3, y3) {
      var p = projection2([x3, y3]);
      this.stream.point(p[0], p[1]);
    }
  }), postclip = identity_default3, cache, cacheStream;
  function reset() {
    kx = k * sx;
    ky = k * sy;
    cache = cacheStream = null;
    return projection2;
  }
  function projection2(p) {
    var x3 = p[0] * kx, y3 = p[1] * ky;
    if (alpha) {
      var t = y3 * ca3 - x3 * sa;
      x3 = x3 * ca3 + y3 * sa;
      y3 = t;
    }
    return [x3 + tx, y3 + ty];
  }
  projection2.invert = function(p) {
    var x3 = p[0] - tx, y3 = p[1] - ty;
    if (alpha) {
      var t = y3 * ca3 + x3 * sa;
      x3 = x3 * ca3 - y3 * sa;
      y3 = t;
    }
    return [x3 / kx, y3 / ky];
  };
  projection2.stream = function(stream) {
    return cache && cacheStream === stream ? cache : cache = transform(postclip(cacheStream = stream));
  };
  projection2.postclip = function(_) {
    return arguments.length ? (postclip = _, x06 = y06 = x12 = y12 = null, reset()) : postclip;
  };
  projection2.clipExtent = function(_) {
    return arguments.length ? (postclip = _ == null ? (x06 = y06 = x12 = y12 = null, identity_default3) : clipRectangle(x06 = +_[0][0], y06 = +_[0][1], x12 = +_[1][0], y12 = +_[1][1]), reset()) : x06 == null ? null : [[x06, y06], [x12, y12]];
  };
  projection2.scale = function(_) {
    return arguments.length ? (k = +_, reset()) : k;
  };
  projection2.translate = function(_) {
    return arguments.length ? (tx = +_[0], ty = +_[1], reset()) : [tx, ty];
  };
  projection2.angle = function(_) {
    return arguments.length ? (alpha = _ % 360 * radians2, sa = sin2(alpha), ca3 = cos2(alpha), reset()) : alpha * degrees3;
  };
  projection2.reflectX = function(_) {
    return arguments.length ? (sx = _ ? -1 : 1, reset()) : sx < 0;
  };
  projection2.reflectY = function(_) {
    return arguments.length ? (sy = _ ? -1 : 1, reset()) : sy < 0;
  };
  projection2.fitExtent = function(extent2, object2) {
    return fitExtent(projection2, extent2, object2);
  };
  projection2.fitSize = function(size, object2) {
    return fitSize(projection2, size, object2);
  };
  projection2.fitWidth = function(width, object2) {
    return fitWidth(projection2, width, object2);
  };
  projection2.fitHeight = function(height, object2) {
    return fitHeight(projection2, height, object2);
  };
  return projection2;
}
// node_modules/d3-geo/src/projection/naturalEarth1.js
function naturalEarth1Raw(lambda, phi) {
  var phi2 = phi * phi, phi4 = phi2 * phi2;
  return [
    lambda * (0.8707 - 0.131979 * phi2 + phi4 * (-0.013791 + phi4 * (0.003971 * phi2 - 0.001529 * phi4))),
    phi * (1.007226 + phi2 * (0.015085 + phi4 * (-0.044475 + 0.028874 * phi2 - 0.005916 * phi4)))
  ];
}
naturalEarth1Raw.invert = function(x3, y3) {
  var phi = y3, i = 25, delta;
  do {
    var phi2 = phi * phi, phi4 = phi2 * phi2;
    phi -= delta = (phi * (1.007226 + phi2 * (0.015085 + phi4 * (-0.044475 + 0.028874 * phi2 - 0.005916 * phi4))) - y3) / (1.007226 + phi2 * (0.015085 * 3 + phi4 * (-0.044475 * 7 + 0.028874 * 9 * phi2 - 0.005916 * 11 * phi4)));
  } while (abs3(delta) > epsilon8 && --i > 0);
  return [
    x3 / (0.8707 + (phi2 = phi * phi) * (-0.131979 + phi2 * (-0.013791 + phi2 * phi2 * phi2 * (0.003971 - 0.001529 * phi2)))),
    phi
  ];
};
function naturalEarth1_default() {
  return projection(naturalEarth1Raw).scale(175.295);
}
// node_modules/d3-geo/src/projection/orthographic.js
function orthographicRaw(x3, y3) {
  return [cos2(y3) * sin2(x3), sin2(y3)];
}
orthographicRaw.invert = azimuthalInvert(asin);
function orthographic_default() {
  return projection(orthographicRaw).scale(249.5).clipAngle(90 + epsilon8);
}
// node_modules/d3-geo/src/projection/stereographic.js
function stereographicRaw(x3, y3) {
  var cy = cos2(y3), k = 1 + cos2(x3) * cy;
  return [cy * sin2(x3) / k, sin2(y3) / k];
}
stereographicRaw.invert = azimuthalInvert(function(z) {
  return 2 * atan(z);
});
function stereographic_default() {
  return projection(stereographicRaw).scale(250).clipAngle(142);
}
// node_modules/d3-geo/src/projection/transverseMercator.js
function transverseMercatorRaw(lambda, phi) {
  return [log(tan((halfPi3 + phi) / 2)), -lambda];
}
transverseMercatorRaw.invert = function(x3, y3) {
  return [-y3, 2 * atan(exp(x3)) - halfPi3];
};
function transverseMercator_default() {
  var m2 = mercatorProjection(transverseMercatorRaw), center2 = m2.center, rotate = m2.rotate;
  m2.center = function(_) {
    return arguments.length ? center2([-_[1], _[0]]) : (_ = center2(), [_[1], -_[0]]);
  };
  m2.rotate = function(_) {
    return arguments.length ? rotate([_[0], _[1], _.length > 2 ? _[2] + 90 : 90]) : (_ = rotate(), [_[0], _[1], _[2] - 90]);
  };
  return rotate([0, 0, 90]).scale(159.155);
}
// node_modules/d3-hierarchy/src/cluster.js
function defaultSeparation(a2, b) {
  return a2.parent === b.parent ? 1 : 2;
}
function meanX(children2) {
  return children2.reduce(meanXReduce, 0) / children2.length;
}
function meanXReduce(x3, c2) {
  return x3 + c2.x;
}
function maxY(children2) {
  return 1 + children2.reduce(maxYReduce, 0);
}
function maxYReduce(y3, c2) {
  return Math.max(y3, c2.y);
}
function leafLeft(node) {
  var children2;
  while (children2 = node.children)
    node = children2[0];
  return node;
}
function leafRight(node) {
  var children2;
  while (children2 = node.children)
    node = children2[children2.length - 1];
  return node;
}
function cluster_default() {
  var separation = defaultSeparation, dx = 1, dy = 1, nodeSize = false;
  function cluster(root3) {
    var previousNode, x3 = 0;
    root3.eachAfter(function(node) {
      var children2 = node.children;
      if (children2) {
        node.x = meanX(children2);
        node.y = maxY(children2);
      } else {
        node.x = previousNode ? x3 += separation(node, previousNode) : 0;
        node.y = 0;
        previousNode = node;
      }
    });
    var left2 = leafLeft(root3), right2 = leafRight(root3), x06 = left2.x - separation(left2, right2) / 2, x12 = right2.x + separation(right2, left2) / 2;
    return root3.eachAfter(nodeSize ? function(node) {
      node.x = (node.x - root3.x) * dx;
      node.y = (root3.y - node.y) * dy;
    } : function(node) {
      node.x = (node.x - x06) / (x12 - x06) * dx;
      node.y = (1 - (root3.y ? node.y / root3.y : 1)) * dy;
    });
  }
  cluster.separation = function(x3) {
    return arguments.length ? (separation = x3, cluster) : separation;
  };
  cluster.size = function(x3) {
    return arguments.length ? (nodeSize = false, dx = +x3[0], dy = +x3[1], cluster) : nodeSize ? null : [dx, dy];
  };
  cluster.nodeSize = function(x3) {
    return arguments.length ? (nodeSize = true, dx = +x3[0], dy = +x3[1], cluster) : nodeSize ? [dx, dy] : null;
  };
  return cluster;
}
// node_modules/d3-hierarchy/src/hierarchy/count.js
function count2(node) {
  var sum3 = 0, children2 = node.children, i = children2 && children2.length;
  if (!i)
    sum3 = 1;
  else
    while (--i >= 0)
      sum3 += children2[i].value;
  node.value = sum3;
}
function count_default() {
  return this.eachAfter(count2);
}

// node_modules/d3-hierarchy/src/hierarchy/each.js
function each_default2(callback, that) {
  let index3 = -1;
  for (const node of this) {
    callback.call(that, node, ++index3, this);
  }
  return this;
}

// node_modules/d3-hierarchy/src/hierarchy/eachBefore.js
function eachBefore_default(callback, that) {
  var node = this, nodes = [node], children2, i, index3 = -1;
  while (node = nodes.pop()) {
    callback.call(that, node, ++index3, this);
    if (children2 = node.children) {
      for (i = children2.length - 1;i >= 0; --i) {
        nodes.push(children2[i]);
      }
    }
  }
  return this;
}

// node_modules/d3-hierarchy/src/hierarchy/eachAfter.js
function eachAfter_default(callback, that) {
  var node = this, nodes = [node], next = [], children2, i, n, index3 = -1;
  while (node = nodes.pop()) {
    next.push(node);
    if (children2 = node.children) {
      for (i = 0, n = children2.length;i < n; ++i) {
        nodes.push(children2[i]);
      }
    }
  }
  while (node = next.pop()) {
    callback.call(that, node, ++index3, this);
  }
  return this;
}

// node_modules/d3-hierarchy/src/hierarchy/find.js
function find_default2(callback, that) {
  let index3 = -1;
  for (const node of this) {
    if (callback.call(that, node, ++index3, this)) {
      return node;
    }
  }
}

// node_modules/d3-hierarchy/src/hierarchy/sum.js
function sum_default(value) {
  return this.eachAfter(function(node) {
    var sum3 = +value(node.data) || 0, children2 = node.children, i = children2 && children2.length;
    while (--i >= 0)
      sum3 += children2[i].value;
    node.value = sum3;
  });
}

// node_modules/d3-hierarchy/src/hierarchy/sort.js
function sort_default2(compare) {
  return this.eachBefore(function(node) {
    if (node.children) {
      node.children.sort(compare);
    }
  });
}

// node_modules/d3-hierarchy/src/hierarchy/path.js
function path_default2(end) {
  var start2 = this, ancestor = leastCommonAncestor(start2, end), nodes = [start2];
  while (start2 !== ancestor) {
    start2 = start2.parent;
    nodes.push(start2);
  }
  var k = nodes.length;
  while (end !== ancestor) {
    nodes.splice(k, 0, end);
    end = end.parent;
  }
  return nodes;
}
function leastCommonAncestor(a2, b) {
  if (a2 === b)
    return a2;
  var aNodes = a2.ancestors(), bNodes = b.ancestors(), c2 = null;
  a2 = aNodes.pop();
  b = bNodes.pop();
  while (a2 === b) {
    c2 = a2;
    a2 = aNodes.pop();
    b = bNodes.pop();
  }
  return c2;
}

// node_modules/d3-hierarchy/src/hierarchy/ancestors.js
function ancestors_default() {
  var node = this, nodes = [node];
  while (node = node.parent) {
    nodes.push(node);
  }
  return nodes;
}

// node_modules/d3-hierarchy/src/hierarchy/descendants.js
function descendants_default() {
  return Array.from(this);
}

// node_modules/d3-hierarchy/src/hierarchy/leaves.js
function leaves_default() {
  var leaves = [];
  this.eachBefore(function(node) {
    if (!node.children) {
      leaves.push(node);
    }
  });
  return leaves;
}

// node_modules/d3-hierarchy/src/hierarchy/links.js
function links_default() {
  var root3 = this, links = [];
  root3.each(function(node) {
    if (node !== root3) {
      links.push({ source: node.parent, target: node });
    }
  });
  return links;
}

// node_modules/d3-hierarchy/src/hierarchy/iterator.js
function* iterator_default2() {
  var node = this, current, next = [node], children2, i, n;
  do {
    current = next.reverse(), next = [];
    while (node = current.pop()) {
      yield node;
      if (children2 = node.children) {
        for (i = 0, n = children2.length;i < n; ++i) {
          next.push(children2[i]);
        }
      }
    }
  } while (next.length);
}

// node_modules/d3-hierarchy/src/hierarchy/index.js
function hierarchy(data, children2) {
  if (data instanceof Map) {
    data = [undefined, data];
    if (children2 === undefined)
      children2 = mapChildren;
  } else if (children2 === undefined) {
    children2 = objectChildren;
  }
  var root3 = new Node(data), node, nodes = [root3], child, childs, i, n;
  while (node = nodes.pop()) {
    if ((childs = children2(node.data)) && (n = (childs = Array.from(childs)).length)) {
      node.children = childs;
      for (i = n - 1;i >= 0; --i) {
        nodes.push(child = childs[i] = new Node(childs[i]));
        child.parent = node;
        child.depth = node.depth + 1;
      }
    }
  }
  return root3.eachBefore(computeHeight);
}
function node_copy() {
  return hierarchy(this).eachBefore(copyData);
}
function objectChildren(d) {
  return d.children;
}
function mapChildren(d) {
  return Array.isArray(d) ? d[1] : null;
}
function copyData(node) {
  if (node.data.value !== undefined)
    node.value = node.data.value;
  node.data = node.data.data;
}
function computeHeight(node) {
  var height = 0;
  do
    node.height = height;
  while ((node = node.parent) && node.height < ++height);
}
function Node(data) {
  this.data = data;
  this.depth = this.height = 0;
  this.parent = null;
}
Node.prototype = hierarchy.prototype = {
  constructor: Node,
  count: count_default,
  each: each_default2,
  eachAfter: eachAfter_default,
  eachBefore: eachBefore_default,
  find: find_default2,
  sum: sum_default,
  sort: sort_default2,
  path: path_default2,
  ancestors: ancestors_default,
  descendants: descendants_default,
  leaves: leaves_default,
  links: links_default,
  copy: node_copy,
  [Symbol.iterator]: iterator_default2
};
// node_modules/d3-hierarchy/src/accessors.js
function optional(f) {
  return f == null ? null : required(f);
}
function required(f) {
  if (typeof f !== "function")
    throw new Error;
  return f;
}

// node_modules/d3-hierarchy/src/constant.js
function constantZero() {
  return 0;
}
function constant_default9(x3) {
  return function() {
    return x3;
  };
}

// node_modules/d3-hierarchy/src/lcg.js
var a2 = 1664525;
var c2 = 1013904223;
var m2 = 4294967296;
function lcg_default2() {
  let s = 1;
  return () => (s = (a2 * s + c2) % m2) / m2;
}

// node_modules/d3-hierarchy/src/array.js
function array_default2(x3) {
  return typeof x3 === "object" && "length" in x3 ? x3 : Array.from(x3);
}
function shuffle(array4, random) {
  let m3 = array4.length, t, i;
  while (m3) {
    i = random() * m3-- | 0;
    t = array4[m3];
    array4[m3] = array4[i];
    array4[i] = t;
  }
  return array4;
}

// node_modules/d3-hierarchy/src/pack/enclose.js
function enclose_default(circles) {
  return packEncloseRandom(circles, lcg_default2());
}
function packEncloseRandom(circles, random) {
  var i = 0, n = (circles = shuffle(Array.from(circles), random)).length, B3 = [], p, e;
  while (i < n) {
    p = circles[i];
    if (e && enclosesWeak(e, p))
      ++i;
    else
      e = encloseBasis(B3 = extendBasis(B3, p)), i = 0;
  }
  return e;
}
function extendBasis(B3, p) {
  var i, j;
  if (enclosesWeakAll(p, B3))
    return [p];
  for (i = 0;i < B3.length; ++i) {
    if (enclosesNot(p, B3[i]) && enclosesWeakAll(encloseBasis2(B3[i], p), B3)) {
      return [B3[i], p];
    }
  }
  for (i = 0;i < B3.length - 1; ++i) {
    for (j = i + 1;j < B3.length; ++j) {
      if (enclosesNot(encloseBasis2(B3[i], B3[j]), p) && enclosesNot(encloseBasis2(B3[i], p), B3[j]) && enclosesNot(encloseBasis2(B3[j], p), B3[i]) && enclosesWeakAll(encloseBasis3(B3[i], B3[j], p), B3)) {
        return [B3[i], B3[j], p];
      }
    }
  }
  throw new Error;
}
function enclosesNot(a3, b) {
  var dr = a3.r - b.r, dx = b.x - a3.x, dy = b.y - a3.y;
  return dr < 0 || dr * dr < dx * dx + dy * dy;
}
function enclosesWeak(a3, b) {
  var dr = a3.r - b.r + Math.max(a3.r, b.r, 1) * 0.000000001, dx = b.x - a3.x, dy = b.y - a3.y;
  return dr > 0 && dr * dr > dx * dx + dy * dy;
}
function enclosesWeakAll(a3, B3) {
  for (var i = 0;i < B3.length; ++i) {
    if (!enclosesWeak(a3, B3[i])) {
      return false;
    }
  }
  return true;
}
function encloseBasis(B3) {
  switch (B3.length) {
    case 1:
      return encloseBasis1(B3[0]);
    case 2:
      return encloseBasis2(B3[0], B3[1]);
    case 3:
      return encloseBasis3(B3[0], B3[1], B3[2]);
  }
}
function encloseBasis1(a3) {
  return {
    x: a3.x,
    y: a3.y,
    r: a3.r
  };
}
function encloseBasis2(a3, b) {
  var { x: x12, y: y12, r: r1 } = a3, x22 = b.x, y22 = b.y, r2 = b.r, x21 = x22 - x12, y21 = y22 - y12, r21 = r2 - r1, l = Math.sqrt(x21 * x21 + y21 * y21);
  return {
    x: (x12 + x22 + x21 / l * r21) / 2,
    y: (y12 + y22 + y21 / l * r21) / 2,
    r: (l + r1 + r2) / 2
  };
}
function encloseBasis3(a3, b, c3) {
  var { x: x12, y: y12, r: r1 } = a3, x22 = b.x, y22 = b.y, r2 = b.r, x3 = c3.x, y3 = c3.y, r3 = c3.r, a22 = x12 - x22, a32 = x12 - x3, b22 = y12 - y22, b32 = y12 - y3, c22 = r2 - r1, c32 = r3 - r1, d1 = x12 * x12 + y12 * y12 - r1 * r1, d2 = d1 - x22 * x22 - y22 * y22 + r2 * r2, d3 = d1 - x3 * x3 - y3 * y3 + r3 * r3, ab4 = a32 * b22 - a22 * b32, xa = (b22 * d3 - b32 * d2) / (ab4 * 2) - x12, xb = (b32 * c22 - b22 * c32) / ab4, ya = (a32 * d2 - a22 * d3) / (ab4 * 2) - y12, yb = (a22 * c32 - a32 * c22) / ab4, A5 = xb * xb + yb * yb - 1, B3 = 2 * (r1 + xa * xb + ya * yb), C3 = xa * xa + ya * ya - r1 * r1, r = -(Math.abs(A5) > 0.000001 ? (B3 + Math.sqrt(B3 * B3 - 4 * A5 * C3)) / (2 * A5) : C3 / B3);
  return {
    x: x12 + xa + xb * r,
    y: y12 + ya + yb * r,
    r
  };
}

// node_modules/d3-hierarchy/src/pack/siblings.js
function place(b, a3, c3) {
  var dx = b.x - a3.x, x3, a22, dy = b.y - a3.y, y3, b22, d2 = dx * dx + dy * dy;
  if (d2) {
    a22 = a3.r + c3.r, a22 *= a22;
    b22 = b.r + c3.r, b22 *= b22;
    if (a22 > b22) {
      x3 = (d2 + b22 - a22) / (2 * d2);
      y3 = Math.sqrt(Math.max(0, b22 / d2 - x3 * x3));
      c3.x = b.x - x3 * dx - y3 * dy;
      c3.y = b.y - x3 * dy + y3 * dx;
    } else {
      x3 = (d2 + a22 - b22) / (2 * d2);
      y3 = Math.sqrt(Math.max(0, a22 / d2 - x3 * x3));
      c3.x = a3.x + x3 * dx - y3 * dy;
      c3.y = a3.y + x3 * dy + y3 * dx;
    }
  } else {
    c3.x = a3.x + c3.r;
    c3.y = a3.y;
  }
}
function intersects(a3, b) {
  var dr = a3.r + b.r - 0.000001, dx = b.x - a3.x, dy = b.y - a3.y;
  return dr > 0 && dr * dr > dx * dx + dy * dy;
}
function score(node) {
  var a3 = node._, b = node.next._, ab4 = a3.r + b.r, dx = (a3.x * b.r + b.x * a3.r) / ab4, dy = (a3.y * b.r + b.y * a3.r) / ab4;
  return dx * dx + dy * dy;
}
function Node2(circle) {
  this._ = circle;
  this.next = null;
  this.previous = null;
}
function packSiblingsRandom(circles, random) {
  if (!(n = (circles = array_default2(circles)).length))
    return 0;
  var a3, b, c3, n, aa2, ca3, i, j, k, sj, sk;
  a3 = circles[0], a3.x = 0, a3.y = 0;
  if (!(n > 1))
    return a3.r;
  b = circles[1], a3.x = -b.r, b.x = a3.r, b.y = 0;
  if (!(n > 2))
    return a3.r + b.r;
  place(b, a3, c3 = circles[2]);
  a3 = new Node2(a3), b = new Node2(b), c3 = new Node2(c3);
  a3.next = c3.previous = b;
  b.next = a3.previous = c3;
  c3.next = b.previous = a3;
  pack:
    for (i = 3;i < n; ++i) {
      place(a3._, b._, c3 = circles[i]), c3 = new Node2(c3);
      j = b.next, k = a3.previous, sj = b._.r, sk = a3._.r;
      do {
        if (sj <= sk) {
          if (intersects(j._, c3._)) {
            b = j, a3.next = b, b.previous = a3, --i;
            continue pack;
          }
          sj += j._.r, j = j.next;
        } else {
          if (intersects(k._, c3._)) {
            a3 = k, a3.next = b, b.previous = a3, --i;
            continue pack;
          }
          sk += k._.r, k = k.previous;
        }
      } while (j !== k.next);
      c3.previous = a3, c3.next = b, a3.next = b.previous = b = c3;
      aa2 = score(a3);
      while ((c3 = c3.next) !== b) {
        if ((ca3 = score(c3)) < aa2) {
          a3 = c3, aa2 = ca3;
        }
      }
      b = a3.next;
    }
  a3 = [b._], c3 = b;
  while ((c3 = c3.next) !== b)
    a3.push(c3._);
  c3 = packEncloseRandom(a3, random);
  for (i = 0;i < n; ++i)
    a3 = circles[i], a3.x -= c3.x, a3.y -= c3.y;
  return c3.r;
}
function siblings_default(circles) {
  packSiblingsRandom(circles, lcg_default2());
  return circles;
}

// node_modules/d3-hierarchy/src/pack/index.js
function defaultRadius2(d) {
  return Math.sqrt(d.value);
}
function pack_default() {
  var radius = null, dx = 1, dy = 1, padding = constantZero;
  function pack(root3) {
    const random = lcg_default2();
    root3.x = dx / 2, root3.y = dy / 2;
    if (radius) {
      root3.eachBefore(radiusLeaf(radius)).eachAfter(packChildrenRandom(padding, 0.5, random)).eachBefore(translateChild(1));
    } else {
      root3.eachBefore(radiusLeaf(defaultRadius2)).eachAfter(packChildrenRandom(constantZero, 1, random)).eachAfter(packChildrenRandom(padding, root3.r / Math.min(dx, dy), random)).eachBefore(translateChild(Math.min(dx, dy) / (2 * root3.r)));
    }
    return root3;
  }
  pack.radius = function(x3) {
    return arguments.length ? (radius = optional(x3), pack) : radius;
  };
  pack.size = function(x3) {
    return arguments.length ? (dx = +x3[0], dy = +x3[1], pack) : [dx, dy];
  };
  pack.padding = function(x3) {
    return arguments.length ? (padding = typeof x3 === "function" ? x3 : constant_default9(+x3), pack) : padding;
  };
  return pack;
}
function radiusLeaf(radius) {
  return function(node) {
    if (!node.children) {
      node.r = Math.max(0, +radius(node) || 0);
    }
  };
}
function packChildrenRandom(padding, k, random) {
  return function(node) {
    if (children2 = node.children) {
      var children2, i, n = children2.length, r = padding(node) * k || 0, e;
      if (r)
        for (i = 0;i < n; ++i)
          children2[i].r += r;
      e = packSiblingsRandom(children2, random);
      if (r)
        for (i = 0;i < n; ++i)
          children2[i].r -= r;
      node.r = e + r;
    }
  };
}
function translateChild(k) {
  return function(node) {
    var parent = node.parent;
    node.r *= k;
    if (parent) {
      node.x = parent.x + k * node.x;
      node.y = parent.y + k * node.y;
    }
  };
}
// node_modules/d3-hierarchy/src/treemap/round.js
function round_default2(node) {
  node.x0 = Math.round(node.x0);
  node.y0 = Math.round(node.y0);
  node.x1 = Math.round(node.x1);
  node.y1 = Math.round(node.y1);
}

// node_modules/d3-hierarchy/src/treemap/dice.js
function dice_default(parent, x06, y06, x12, y12) {
  var nodes = parent.children, node, i = -1, n = nodes.length, k = parent.value && (x12 - x06) / parent.value;
  while (++i < n) {
    node = nodes[i], node.y0 = y06, node.y1 = y12;
    node.x0 = x06, node.x1 = x06 += node.value * k;
  }
}

// node_modules/d3-hierarchy/src/partition.js
function partition_default() {
  var dx = 1, dy = 1, padding = 0, round = false;
  function partition(root3) {
    var n = root3.height + 1;
    root3.x0 = root3.y0 = padding;
    root3.x1 = dx;
    root3.y1 = dy / n;
    root3.eachBefore(positionNode(dy, n));
    if (round)
      root3.eachBefore(round_default2);
    return root3;
  }
  function positionNode(dy2, n) {
    return function(node) {
      if (node.children) {
        dice_default(node, node.x0, dy2 * (node.depth + 1) / n, node.x1, dy2 * (node.depth + 2) / n);
      }
      var { x0: x06, y0: y06 } = node, x12 = node.x1 - padding, y12 = node.y1 - padding;
      if (x12 < x06)
        x06 = x12 = (x06 + x12) / 2;
      if (y12 < y06)
        y06 = y12 = (y06 + y12) / 2;
      node.x0 = x06;
      node.y0 = y06;
      node.x1 = x12;
      node.y1 = y12;
    };
  }
  partition.round = function(x3) {
    return arguments.length ? (round = !!x3, partition) : round;
  };
  partition.size = function(x3) {
    return arguments.length ? (dx = +x3[0], dy = +x3[1], partition) : [dx, dy];
  };
  partition.padding = function(x3) {
    return arguments.length ? (padding = +x3, partition) : padding;
  };
  return partition;
}
// node_modules/d3-hierarchy/src/stratify.js
var preroot = { depth: -1 };
var ambiguous = {};
var imputed = {};
function defaultId(d) {
  return d.id;
}
function defaultParentId(d) {
  return d.parentId;
}
function stratify_default() {
  var id2 = defaultId, parentId = defaultParentId, path2;
  function stratify(data) {
    var nodes = Array.from(data), currentId = id2, currentParentId = parentId, n, d, i, root3, parent, node, nodeId, nodeKey, nodeByKey = new Map;
    if (path2 != null) {
      const I = nodes.map((d2, i2) => normalize(path2(d2, i2, data)));
      const P = I.map(parentof);
      const S = new Set(I).add("");
      for (const i2 of P) {
        if (!S.has(i2)) {
          S.add(i2);
          I.push(i2);
          P.push(parentof(i2));
          nodes.push(imputed);
        }
      }
      currentId = (_, i2) => I[i2];
      currentParentId = (_, i2) => P[i2];
    }
    for (i = 0, n = nodes.length;i < n; ++i) {
      d = nodes[i], node = nodes[i] = new Node(d);
      if ((nodeId = currentId(d, i, data)) != null && (nodeId += "")) {
        nodeKey = node.id = nodeId;
        nodeByKey.set(nodeKey, nodeByKey.has(nodeKey) ? ambiguous : node);
      }
      if ((nodeId = currentParentId(d, i, data)) != null && (nodeId += "")) {
        node.parent = nodeId;
      }
    }
    for (i = 0;i < n; ++i) {
      node = nodes[i];
      if (nodeId = node.parent) {
        parent = nodeByKey.get(nodeId);
        if (!parent)
          throw new Error("missing: " + nodeId);
        if (parent === ambiguous)
          throw new Error("ambiguous: " + nodeId);
        if (parent.children)
          parent.children.push(node);
        else
          parent.children = [node];
        node.parent = parent;
      } else {
        if (root3)
          throw new Error("multiple roots");
        root3 = node;
      }
    }
    if (!root3)
      throw new Error("no root");
    if (path2 != null) {
      while (root3.data === imputed && root3.children.length === 1) {
        root3 = root3.children[0], --n;
      }
      for (let i2 = nodes.length - 1;i2 >= 0; --i2) {
        node = nodes[i2];
        if (node.data !== imputed)
          break;
        node.data = null;
      }
    }
    root3.parent = preroot;
    root3.eachBefore(function(node2) {
      node2.depth = node2.parent.depth + 1;
      --n;
    }).eachBefore(computeHeight);
    root3.parent = null;
    if (n > 0)
      throw new Error("cycle");
    return root3;
  }
  stratify.id = function(x3) {
    return arguments.length ? (id2 = optional(x3), stratify) : id2;
  };
  stratify.parentId = function(x3) {
    return arguments.length ? (parentId = optional(x3), stratify) : parentId;
  };
  stratify.path = function(x3) {
    return arguments.length ? (path2 = optional(x3), stratify) : path2;
  };
  return stratify;
}
function normalize(path2) {
  path2 = `${path2}`;
  let i = path2.length;
  if (slash(path2, i - 1) && !slash(path2, i - 2))
    path2 = path2.slice(0, -1);
  return path2[0] === "/" ? path2 : `/${path2}`;
}
function parentof(path2) {
  let i = path2.length;
  if (i < 2)
    return "";
  while (--i > 1)
    if (slash(path2, i))
      break;
  return path2.slice(0, i);
}
function slash(path2, i) {
  if (path2[i] === "/") {
    let k = 0;
    while (i > 0 && path2[--i] === "\\")
      ++k;
    if ((k & 1) === 0)
      return true;
  }
  return false;
}
// node_modules/d3-hierarchy/src/tree.js
function defaultSeparation2(a3, b) {
  return a3.parent === b.parent ? 1 : 2;
}
function nextLeft(v2) {
  var children2 = v2.children;
  return children2 ? children2[0] : v2.t;
}
function nextRight(v2) {
  var children2 = v2.children;
  return children2 ? children2[children2.length - 1] : v2.t;
}
function moveSubtree(wm, wp, shift) {
  var change = shift / (wp.i - wm.i);
  wp.c -= change;
  wp.s += shift;
  wm.c += change;
  wp.z += shift;
  wp.m += shift;
}
function executeShifts(v2) {
  var shift = 0, change = 0, children2 = v2.children, i = children2.length, w;
  while (--i >= 0) {
    w = children2[i];
    w.z += shift;
    w.m += shift;
    shift += w.s + (change += w.c);
  }
}
function nextAncestor(vim, v2, ancestor) {
  return vim.a.parent === v2.parent ? vim.a : ancestor;
}
function TreeNode(node, i) {
  this._ = node;
  this.parent = null;
  this.children = null;
  this.A = null;
  this.a = this;
  this.z = 0;
  this.m = 0;
  this.c = 0;
  this.s = 0;
  this.t = null;
  this.i = i;
}
TreeNode.prototype = Object.create(Node.prototype);
function treeRoot(root3) {
  var tree = new TreeNode(root3, 0), node, nodes = [tree], child, children2, i, n;
  while (node = nodes.pop()) {
    if (children2 = node._.children) {
      node.children = new Array(n = children2.length);
      for (i = n - 1;i >= 0; --i) {
        nodes.push(child = node.children[i] = new TreeNode(children2[i], i));
        child.parent = node;
      }
    }
  }
  (tree.parent = new TreeNode(null, 0)).children = [tree];
  return tree;
}
function tree_default() {
  var separation = defaultSeparation2, dx = 1, dy = 1, nodeSize = null;
  function tree(root3) {
    var t = treeRoot(root3);
    t.eachAfter(firstWalk), t.parent.m = -t.z;
    t.eachBefore(secondWalk);
    if (nodeSize)
      root3.eachBefore(sizeNode);
    else {
      var left2 = root3, right2 = root3, bottom2 = root3;
      root3.eachBefore(function(node) {
        if (node.x < left2.x)
          left2 = node;
        if (node.x > right2.x)
          right2 = node;
        if (node.depth > bottom2.depth)
          bottom2 = node;
      });
      var s = left2 === right2 ? 1 : separation(left2, right2) / 2, tx = s - left2.x, kx = dx / (right2.x + s + tx), ky = dy / (bottom2.depth || 1);
      root3.eachBefore(function(node) {
        node.x = (node.x + tx) * kx;
        node.y = node.depth * ky;
      });
    }
    return root3;
  }
  function firstWalk(v2) {
    var children2 = v2.children, siblings = v2.parent.children, w = v2.i ? siblings[v2.i - 1] : null;
    if (children2) {
      executeShifts(v2);
      var midpoint = (children2[0].z + children2[children2.length - 1].z) / 2;
      if (w) {
        v2.z = w.z + separation(v2._, w._);
        v2.m = v2.z - midpoint;
      } else {
        v2.z = midpoint;
      }
    } else if (w) {
      v2.z = w.z + separation(v2._, w._);
    }
    v2.parent.A = apportion(v2, w, v2.parent.A || siblings[0]);
  }
  function secondWalk(v2) {
    v2._.x = v2.z + v2.parent.m;
    v2.m += v2.parent.m;
  }
  function apportion(v2, w, ancestor) {
    if (w) {
      var vip = v2, vop = v2, vim = w, vom = vip.parent.children[0], sip = vip.m, sop = vop.m, sim = vim.m, som = vom.m, shift;
      while (vim = nextRight(vim), vip = nextLeft(vip), vim && vip) {
        vom = nextLeft(vom);
        vop = nextRight(vop);
        vop.a = v2;
        shift = vim.z + sim - vip.z - sip + separation(vim._, vip._);
        if (shift > 0) {
          moveSubtree(nextAncestor(vim, v2, ancestor), v2, shift);
          sip += shift;
          sop += shift;
        }
        sim += vim.m;
        sip += vip.m;
        som += vom.m;
        sop += vop.m;
      }
      if (vim && !nextRight(vop)) {
        vop.t = vim;
        vop.m += sim - sop;
      }
      if (vip && !nextLeft(vom)) {
        vom.t = vip;
        vom.m += sip - som;
        ancestor = v2;
      }
    }
    return ancestor;
  }
  function sizeNode(node) {
    node.x *= dx;
    node.y = node.depth * dy;
  }
  tree.separation = function(x3) {
    return arguments.length ? (separation = x3, tree) : separation;
  };
  tree.size = function(x3) {
    return arguments.length ? (nodeSize = false, dx = +x3[0], dy = +x3[1], tree) : nodeSize ? null : [dx, dy];
  };
  tree.nodeSize = function(x3) {
    return arguments.length ? (nodeSize = true, dx = +x3[0], dy = +x3[1], tree) : nodeSize ? [dx, dy] : null;
  };
  return tree;
}
// node_modules/d3-hierarchy/src/treemap/slice.js
function slice_default(parent, x06, y06, x12, y12) {
  var nodes = parent.children, node, i = -1, n = nodes.length, k = parent.value && (y12 - y06) / parent.value;
  while (++i < n) {
    node = nodes[i], node.x0 = x06, node.x1 = x12;
    node.y0 = y06, node.y1 = y06 += node.value * k;
  }
}

// node_modules/d3-hierarchy/src/treemap/squarify.js
var phi = (1 + Math.sqrt(5)) / 2;
function squarifyRatio(ratio, parent, x06, y06, x12, y12) {
  var rows = [], nodes = parent.children, row, nodeValue, i0 = 0, i1 = 0, n = nodes.length, dx, dy, value = parent.value, sumValue, minValue, maxValue, newRatio, minRatio, alpha, beta;
  while (i0 < n) {
    dx = x12 - x06, dy = y12 - y06;
    do
      sumValue = nodes[i1++].value;
    while (!sumValue && i1 < n);
    minValue = maxValue = sumValue;
    alpha = Math.max(dy / dx, dx / dy) / (value * ratio);
    beta = sumValue * sumValue * alpha;
    minRatio = Math.max(maxValue / beta, beta / minValue);
    for (;i1 < n; ++i1) {
      sumValue += nodeValue = nodes[i1].value;
      if (nodeValue < minValue)
        minValue = nodeValue;
      if (nodeValue > maxValue)
        maxValue = nodeValue;
      beta = sumValue * sumValue * alpha;
      newRatio = Math.max(maxValue / beta, beta / minValue);
      if (newRatio > minRatio) {
        sumValue -= nodeValue;
        break;
      }
      minRatio = newRatio;
    }
    rows.push(row = { value: sumValue, dice: dx < dy, children: nodes.slice(i0, i1) });
    if (row.dice)
      dice_default(row, x06, y06, x12, value ? y06 += dy * sumValue / value : y12);
    else
      slice_default(row, x06, y06, value ? x06 += dx * sumValue / value : x12, y12);
    value -= sumValue, i0 = i1;
  }
  return rows;
}
var squarify_default = function custom10(ratio) {
  function squarify(parent, x06, y06, x12, y12) {
    squarifyRatio(ratio, parent, x06, y06, x12, y12);
  }
  squarify.ratio = function(x3) {
    return custom10((x3 = +x3) > 1 ? x3 : 1);
  };
  return squarify;
}(phi);

// node_modules/d3-hierarchy/src/treemap/index.js
function treemap_default() {
  var tile = squarify_default, round = false, dx = 1, dy = 1, paddingStack = [0], paddingInner = constantZero, paddingTop = constantZero, paddingRight = constantZero, paddingBottom = constantZero, paddingLeft = constantZero;
  function treemap(root3) {
    root3.x0 = root3.y0 = 0;
    root3.x1 = dx;
    root3.y1 = dy;
    root3.eachBefore(positionNode);
    paddingStack = [0];
    if (round)
      root3.eachBefore(round_default2);
    return root3;
  }
  function positionNode(node) {
    var p = paddingStack[node.depth], x06 = node.x0 + p, y06 = node.y0 + p, x12 = node.x1 - p, y12 = node.y1 - p;
    if (x12 < x06)
      x06 = x12 = (x06 + x12) / 2;
    if (y12 < y06)
      y06 = y12 = (y06 + y12) / 2;
    node.x0 = x06;
    node.y0 = y06;
    node.x1 = x12;
    node.y1 = y12;
    if (node.children) {
      p = paddingStack[node.depth + 1] = paddingInner(node) / 2;
      x06 += paddingLeft(node) - p;
      y06 += paddingTop(node) - p;
      x12 -= paddingRight(node) - p;
      y12 -= paddingBottom(node) - p;
      if (x12 < x06)
        x06 = x12 = (x06 + x12) / 2;
      if (y12 < y06)
        y06 = y12 = (y06 + y12) / 2;
      tile(node, x06, y06, x12, y12);
    }
  }
  treemap.round = function(x3) {
    return arguments.length ? (round = !!x3, treemap) : round;
  };
  treemap.size = function(x3) {
    return arguments.length ? (dx = +x3[0], dy = +x3[1], treemap) : [dx, dy];
  };
  treemap.tile = function(x3) {
    return arguments.length ? (tile = required(x3), treemap) : tile;
  };
  treemap.padding = function(x3) {
    return arguments.length ? treemap.paddingInner(x3).paddingOuter(x3) : treemap.paddingInner();
  };
  treemap.paddingInner = function(x3) {
    return arguments.length ? (paddingInner = typeof x3 === "function" ? x3 : constant_default9(+x3), treemap) : paddingInner;
  };
  treemap.paddingOuter = function(x3) {
    return arguments.length ? treemap.paddingTop(x3).paddingRight(x3).paddingBottom(x3).paddingLeft(x3) : treemap.paddingTop();
  };
  treemap.paddingTop = function(x3) {
    return arguments.length ? (paddingTop = typeof x3 === "function" ? x3 : constant_default9(+x3), treemap) : paddingTop;
  };
  treemap.paddingRight = function(x3) {
    return arguments.length ? (paddingRight = typeof x3 === "function" ? x3 : constant_default9(+x3), treemap) : paddingRight;
  };
  treemap.paddingBottom = function(x3) {
    return arguments.length ? (paddingBottom = typeof x3 === "function" ? x3 : constant_default9(+x3), treemap) : paddingBottom;
  };
  treemap.paddingLeft = function(x3) {
    return arguments.length ? (paddingLeft = typeof x3 === "function" ? x3 : constant_default9(+x3), treemap) : paddingLeft;
  };
  return treemap;
}
// node_modules/d3-hierarchy/src/treemap/binary.js
function binary_default(parent, x06, y06, x12, y12) {
  var nodes = parent.children, i, n = nodes.length, sum3, sums = new Array(n + 1);
  for (sums[0] = sum3 = i = 0;i < n; ++i) {
    sums[i + 1] = sum3 += nodes[i].value;
  }
  partition(0, n, parent.value, x06, y06, x12, y12);
  function partition(i2, j, value, x07, y07, x13, y13) {
    if (i2 >= j - 1) {
      var node = nodes[i2];
      node.x0 = x07, node.y0 = y07;
      node.x1 = x13, node.y1 = y13;
      return;
    }
    var valueOffset = sums[i2], valueTarget = value / 2 + valueOffset, k = i2 + 1, hi = j - 1;
    while (k < hi) {
      var mid = k + hi >>> 1;
      if (sums[mid] < valueTarget)
        k = mid + 1;
      else
        hi = mid;
    }
    if (valueTarget - sums[k - 1] < sums[k] - valueTarget && i2 + 1 < k)
      --k;
    var valueLeft = sums[k] - valueOffset, valueRight = value - valueLeft;
    if (x13 - x07 > y13 - y07) {
      var xk = value ? (x07 * valueRight + x13 * valueLeft) / value : x13;
      partition(i2, k, valueLeft, x07, y07, xk, y13);
      partition(k, j, valueRight, xk, y07, x13, y13);
    } else {
      var yk = value ? (y07 * valueRight + y13 * valueLeft) / value : y13;
      partition(i2, k, valueLeft, x07, y07, x13, yk);
      partition(k, j, valueRight, x07, yk, x13, y13);
    }
  }
}
// node_modules/d3-hierarchy/src/treemap/sliceDice.js
function sliceDice_default(parent, x06, y06, x12, y12) {
  (parent.depth & 1 ? slice_default : dice_default)(parent, x06, y06, x12, y12);
}
// node_modules/d3-hierarchy/src/treemap/resquarify.js
var resquarify_default = function custom11(ratio) {
  function resquarify(parent, x06, y06, x12, y12) {
    if ((rows = parent._squarify) && rows.ratio === ratio) {
      var rows, row, nodes, i, j = -1, n, m3 = rows.length, value = parent.value;
      while (++j < m3) {
        row = rows[j], nodes = row.children;
        for (i = row.value = 0, n = nodes.length;i < n; ++i)
          row.value += nodes[i].value;
        if (row.dice)
          dice_default(row, x06, y06, x12, value ? y06 += (y12 - y06) * row.value / value : y12);
        else
          slice_default(row, x06, y06, value ? x06 += (x12 - x06) * row.value / value : x12, y12);
        value -= row.value;
      }
    } else {
      parent._squarify = rows = squarifyRatio(ratio, parent, x06, y06, x12, y12);
      rows.ratio = ratio;
    }
  }
  resquarify.ratio = function(x3) {
    return custom11((x3 = +x3) > 1 ? x3 : 1);
  };
  return resquarify;
}(phi);
// node_modules/d3-polygon/src/area.js
function area_default4(polygon) {
  var i = -1, n = polygon.length, a3, b = polygon[n - 1], area = 0;
  while (++i < n) {
    a3 = b;
    b = polygon[i];
    area += a3[1] * b[0] - a3[0] * b[1];
  }
  return area / 2;
}
// node_modules/d3-polygon/src/centroid.js
function centroid_default3(polygon) {
  var i = -1, n = polygon.length, x3 = 0, y3 = 0, a3, b = polygon[n - 1], c3, k = 0;
  while (++i < n) {
    a3 = b;
    b = polygon[i];
    k += c3 = a3[0] * b[1] - b[0] * a3[1];
    x3 += (a3[0] + b[0]) * c3;
    y3 += (a3[1] + b[1]) * c3;
  }
  return k *= 3, [x3 / k, y3 / k];
}
// node_modules/d3-polygon/src/cross.js
function cross_default(a3, b, c3) {
  return (b[0] - a3[0]) * (c3[1] - a3[1]) - (b[1] - a3[1]) * (c3[0] - a3[0]);
}

// node_modules/d3-polygon/src/hull.js
function lexicographicOrder(a3, b) {
  return a3[0] - b[0] || a3[1] - b[1];
}
function computeUpperHullIndexes(points) {
  const n = points.length, indexes2 = [0, 1];
  let size = 2, i;
  for (i = 2;i < n; ++i) {
    while (size > 1 && cross_default(points[indexes2[size - 2]], points[indexes2[size - 1]], points[i]) <= 0)
      --size;
    indexes2[size++] = i;
  }
  return indexes2.slice(0, size);
}
function hull_default(points) {
  if ((n = points.length) < 3)
    return null;
  var i, n, sortedPoints = new Array(n), flippedPoints = new Array(n);
  for (i = 0;i < n; ++i)
    sortedPoints[i] = [+points[i][0], +points[i][1], i];
  sortedPoints.sort(lexicographicOrder);
  for (i = 0;i < n; ++i)
    flippedPoints[i] = [sortedPoints[i][0], -sortedPoints[i][1]];
  var upperIndexes = computeUpperHullIndexes(sortedPoints), lowerIndexes = computeUpperHullIndexes(flippedPoints);
  var skipLeft = lowerIndexes[0] === upperIndexes[0], skipRight = lowerIndexes[lowerIndexes.length - 1] === upperIndexes[upperIndexes.length - 1], hull = [];
  for (i = upperIndexes.length - 1;i >= 0; --i)
    hull.push(points[sortedPoints[upperIndexes[i]][2]]);
  for (i = +skipLeft;i < lowerIndexes.length - skipRight; ++i)
    hull.push(points[sortedPoints[lowerIndexes[i]][2]]);
  return hull;
}
// node_modules/d3-polygon/src/contains.js
function contains_default3(polygon, point) {
  var n = polygon.length, p = polygon[n - 1], x3 = point[0], y3 = point[1], x06 = p[0], y06 = p[1], x12, y12, inside = false;
  for (var i = 0;i < n; ++i) {
    p = polygon[i], x12 = p[0], y12 = p[1];
    if (y12 > y3 !== y06 > y3 && x3 < (x06 - x12) * (y3 - y12) / (y06 - y12) + x12)
      inside = !inside;
    x06 = x12, y06 = y12;
  }
  return inside;
}
// node_modules/d3-polygon/src/length.js
function length_default2(polygon) {
  var i = -1, n = polygon.length, b = polygon[n - 1], xa, ya, xb = b[0], yb = b[1], perimeter = 0;
  while (++i < n) {
    xa = xb;
    ya = yb;
    b = polygon[i];
    xb = b[0];
    yb = b[1];
    xa -= xb;
    ya -= yb;
    perimeter += Math.hypot(xa, ya);
  }
  return perimeter;
}
// node_modules/d3-random/src/defaultSource.js
var defaultSource_default = Math.random;

// node_modules/d3-random/src/uniform.js
var uniform_default = function sourceRandomUniform(source) {
  function randomUniform(min3, max4) {
    min3 = min3 == null ? 0 : +min3;
    max4 = max4 == null ? 1 : +max4;
    if (arguments.length === 1)
      max4 = min3, min3 = 0;
    else
      max4 -= min3;
    return function() {
      return source() * max4 + min3;
    };
  }
  randomUniform.source = sourceRandomUniform;
  return randomUniform;
}(defaultSource_default);
// node_modules/d3-random/src/int.js
var int_default = function sourceRandomInt(source) {
  function randomInt(min3, max4) {
    if (arguments.length < 2)
      max4 = min3, min3 = 0;
    min3 = Math.floor(min3);
    max4 = Math.floor(max4) - min3;
    return function() {
      return Math.floor(source() * max4 + min3);
    };
  }
  randomInt.source = sourceRandomInt;
  return randomInt;
}(defaultSource_default);
// node_modules/d3-random/src/normal.js
var normal_default = function sourceRandomNormal(source) {
  function randomNormal(mu, sigma) {
    var x3, r;
    mu = mu == null ? 0 : +mu;
    sigma = sigma == null ? 1 : +sigma;
    return function() {
      var y3;
      if (x3 != null)
        y3 = x3, x3 = null;
      else
        do {
          x3 = source() * 2 - 1;
          y3 = source() * 2 - 1;
          r = x3 * x3 + y3 * y3;
        } while (!r || r > 1);
      return mu + sigma * y3 * Math.sqrt(-2 * Math.log(r) / r);
    };
  }
  randomNormal.source = sourceRandomNormal;
  return randomNormal;
}(defaultSource_default);
// node_modules/d3-random/src/logNormal.js
var logNormal_default = function sourceRandomLogNormal(source) {
  var N = normal_default.source(source);
  function randomLogNormal() {
    var randomNormal = N.apply(this, arguments);
    return function() {
      return Math.exp(randomNormal());
    };
  }
  randomLogNormal.source = sourceRandomLogNormal;
  return randomLogNormal;
}(defaultSource_default);
// node_modules/d3-random/src/irwinHall.js
var irwinHall_default = function sourceRandomIrwinHall(source) {
  function randomIrwinHall(n) {
    if ((n = +n) <= 0)
      return () => 0;
    return function() {
      for (var sum3 = 0, i = n;i > 1; --i)
        sum3 += source();
      return sum3 + i * source();
    };
  }
  randomIrwinHall.source = sourceRandomIrwinHall;
  return randomIrwinHall;
}(defaultSource_default);

// node_modules/d3-random/src/bates.js
var bates_default = function sourceRandomBates(source) {
  var I = irwinHall_default.source(source);
  function randomBates(n) {
    if ((n = +n) === 0)
      return source;
    var randomIrwinHall = I(n);
    return function() {
      return randomIrwinHall() / n;
    };
  }
  randomBates.source = sourceRandomBates;
  return randomBates;
}(defaultSource_default);
// node_modules/d3-random/src/exponential.js
var exponential_default = function sourceRandomExponential(source) {
  function randomExponential(lambda) {
    return function() {
      return -Math.log1p(-source()) / lambda;
    };
  }
  randomExponential.source = sourceRandomExponential;
  return randomExponential;
}(defaultSource_default);
// node_modules/d3-random/src/pareto.js
var pareto_default = function sourceRandomPareto(source) {
  function randomPareto(alpha) {
    if ((alpha = +alpha) < 0)
      throw new RangeError("invalid alpha");
    alpha = 1 / -alpha;
    return function() {
      return Math.pow(1 - source(), alpha);
    };
  }
  randomPareto.source = sourceRandomPareto;
  return randomPareto;
}(defaultSource_default);
// node_modules/d3-random/src/bernoulli.js
var bernoulli_default = function sourceRandomBernoulli(source) {
  function randomBernoulli(p) {
    if ((p = +p) < 0 || p > 1)
      throw new RangeError("invalid p");
    return function() {
      return Math.floor(source() + p);
    };
  }
  randomBernoulli.source = sourceRandomBernoulli;
  return randomBernoulli;
}(defaultSource_default);
// node_modules/d3-random/src/geometric.js
var geometric_default = function sourceRandomGeometric(source) {
  function randomGeometric(p) {
    if ((p = +p) < 0 || p > 1)
      throw new RangeError("invalid p");
    if (p === 0)
      return () => Infinity;
    if (p === 1)
      return () => 1;
    p = Math.log1p(-p);
    return function() {
      return 1 + Math.floor(Math.log1p(-source()) / p);
    };
  }
  randomGeometric.source = sourceRandomGeometric;
  return randomGeometric;
}(defaultSource_default);
// node_modules/d3-random/src/gamma.js
var gamma_default = function sourceRandomGamma(source) {
  var randomNormal = normal_default.source(source)();
  function randomGamma(k, theta) {
    if ((k = +k) < 0)
      throw new RangeError("invalid k");
    if (k === 0)
      return () => 0;
    theta = theta == null ? 1 : +theta;
    if (k === 1)
      return () => -Math.log1p(-source()) * theta;
    var d = (k < 1 ? k + 1 : k) - 1 / 3, c3 = 1 / (3 * Math.sqrt(d)), multiplier = k < 1 ? () => Math.pow(source(), 1 / k) : () => 1;
    return function() {
      do {
        do {
          var x3 = randomNormal(), v2 = 1 + c3 * x3;
        } while (v2 <= 0);
        v2 *= v2 * v2;
        var u4 = 1 - source();
      } while (u4 >= 1 - 0.0331 * x3 * x3 * x3 * x3 && Math.log(u4) >= 0.5 * x3 * x3 + d * (1 - v2 + Math.log(v2)));
      return d * v2 * multiplier() * theta;
    };
  }
  randomGamma.source = sourceRandomGamma;
  return randomGamma;
}(defaultSource_default);

// node_modules/d3-random/src/beta.js
var beta_default = function sourceRandomBeta(source) {
  var G = gamma_default.source(source);
  function randomBeta(alpha, beta) {
    var X3 = G(alpha), Y3 = G(beta);
    return function() {
      var x3 = X3();
      return x3 === 0 ? 0 : x3 / (x3 + Y3());
    };
  }
  randomBeta.source = sourceRandomBeta;
  return randomBeta;
}(defaultSource_default);

// node_modules/d3-random/src/binomial.js
var binomial_default = function sourceRandomBinomial(source) {
  var G = geometric_default.source(source), B3 = beta_default.source(source);
  function randomBinomial(n, p) {
    n = +n;
    if ((p = +p) >= 1)
      return () => n;
    if (p <= 0)
      return () => 0;
    return function() {
      var acc = 0, nn = n, pp = p;
      while (nn * pp > 16 && nn * (1 - pp) > 16) {
        var i = Math.floor((nn + 1) * pp), y3 = B3(i, nn - i + 1)();
        if (y3 <= pp) {
          acc += i;
          nn -= i;
          pp = (pp - y3) / (1 - y3);
        } else {
          nn = i - 1;
          pp /= y3;
        }
      }
      var sign2 = pp < 0.5, pFinal = sign2 ? pp : 1 - pp, g = G(pFinal);
      for (var s = g(), k = 0;s <= nn; ++k)
        s += g();
      return acc + (sign2 ? k : nn - k);
    };
  }
  randomBinomial.source = sourceRandomBinomial;
  return randomBinomial;
}(defaultSource_default);
// node_modules/d3-random/src/weibull.js
var weibull_default = function sourceRandomWeibull(source) {
  function randomWeibull(k, a3, b) {
    var outerFunc;
    if ((k = +k) === 0) {
      outerFunc = (x3) => -Math.log(x3);
    } else {
      k = 1 / k;
      outerFunc = (x3) => Math.pow(x3, k);
    }
    a3 = a3 == null ? 0 : +a3;
    b = b == null ? 1 : +b;
    return function() {
      return a3 + b * outerFunc(-Math.log1p(-source()));
    };
  }
  randomWeibull.source = sourceRandomWeibull;
  return randomWeibull;
}(defaultSource_default);
// node_modules/d3-random/src/cauchy.js
var cauchy_default = function sourceRandomCauchy(source) {
  function randomCauchy(a3, b) {
    a3 = a3 == null ? 0 : +a3;
    b = b == null ? 1 : +b;
    return function() {
      return a3 + b * Math.tan(Math.PI * source());
    };
  }
  randomCauchy.source = sourceRandomCauchy;
  return randomCauchy;
}(defaultSource_default);
// node_modules/d3-random/src/logistic.js
var logistic_default = function sourceRandomLogistic(source) {
  function randomLogistic(a3, b) {
    a3 = a3 == null ? 0 : +a3;
    b = b == null ? 1 : +b;
    return function() {
      var u4 = source();
      return a3 + b * Math.log(u4 / (1 - u4));
    };
  }
  randomLogistic.source = sourceRandomLogistic;
  return randomLogistic;
}(defaultSource_default);
// node_modules/d3-random/src/poisson.js
var poisson_default = function sourceRandomPoisson(source) {
  var G = gamma_default.source(source), B3 = binomial_default.source(source);
  function randomPoisson(lambda) {
    return function() {
      var acc = 0, l = lambda;
      while (l > 16) {
        var n = Math.floor(0.875 * l), t = G(n)();
        if (t > l)
          return acc + B3(n - 1, l / t)();
        acc += n;
        l -= t;
      }
      for (var s = -Math.log1p(-source()), k = 0;s <= l; ++k)
        s -= Math.log1p(-source());
      return acc + k;
    };
  }
  randomPoisson.source = sourceRandomPoisson;
  return randomPoisson;
}(defaultSource_default);
// node_modules/d3-random/src/lcg.js
var mul = 1664525;
var inc = 1013904223;
var eps = 1 / 4294967296;
function lcg(seed = Math.random()) {
  let state = (0 <= seed && seed < 1 ? seed / eps : Math.abs(seed)) | 0;
  return () => (state = mul * state + inc | 0, eps * (state >>> 0));
}
// node_modules/d3-scale/src/init.js
function initRange(domain, range4) {
  switch (arguments.length) {
    case 0:
      break;
    case 1:
      this.range(domain);
      break;
    default:
      this.range(range4).domain(domain);
      break;
  }
  return this;
}
function initInterpolator(domain, interpolator) {
  switch (arguments.length) {
    case 0:
      break;
    case 1: {
      if (typeof domain === "function")
        this.interpolator(domain);
      else
        this.range(domain);
      break;
    }
    default: {
      this.domain(domain);
      if (typeof interpolator === "function")
        this.interpolator(interpolator);
      else
        this.range(interpolator);
      break;
    }
  }
  return this;
}

// node_modules/d3-scale/src/ordinal.js
var implicit = Symbol("implicit");
function ordinal() {
  var index3 = new InternMap, domain = [], range4 = [], unknown = implicit;
  function scale2(d) {
    let i = index3.get(d);
    if (i === undefined) {
      if (unknown !== implicit)
        return unknown;
      index3.set(d, i = domain.push(d) - 1);
    }
    return range4[i % range4.length];
  }
  scale2.domain = function(_) {
    if (!arguments.length)
      return domain.slice();
    domain = [], index3 = new InternMap;
    for (const value of _) {
      if (index3.has(value))
        continue;
      index3.set(value, domain.push(value) - 1);
    }
    return scale2;
  };
  scale2.range = function(_) {
    return arguments.length ? (range4 = Array.from(_), scale2) : range4.slice();
  };
  scale2.unknown = function(_) {
    return arguments.length ? (unknown = _, scale2) : unknown;
  };
  scale2.copy = function() {
    return ordinal(domain, range4).unknown(unknown);
  };
  initRange.apply(scale2, arguments);
  return scale2;
}

// node_modules/d3-scale/src/band.js
function band() {
  var scale2 = ordinal().unknown(undefined), domain = scale2.domain, ordinalRange = scale2.range, r0 = 0, r1 = 1, step, bandwidth, round = false, paddingInner = 0, paddingOuter = 0, align = 0.5;
  delete scale2.unknown;
  function rescale() {
    var n = domain().length, reverse2 = r1 < r0, start2 = reverse2 ? r1 : r0, stop = reverse2 ? r0 : r1;
    step = (stop - start2) / Math.max(1, n - paddingInner + paddingOuter * 2);
    if (round)
      step = Math.floor(step);
    start2 += (stop - start2 - step * (n - paddingInner)) * align;
    bandwidth = step * (1 - paddingInner);
    if (round)
      start2 = Math.round(start2), bandwidth = Math.round(bandwidth);
    var values = range(n).map(function(i) {
      return start2 + step * i;
    });
    return ordinalRange(reverse2 ? values.reverse() : values);
  }
  scale2.domain = function(_) {
    return arguments.length ? (domain(_), rescale()) : domain();
  };
  scale2.range = function(_) {
    return arguments.length ? ([r0, r1] = _, r0 = +r0, r1 = +r1, rescale()) : [r0, r1];
  };
  scale2.rangeRound = function(_) {
    return [r0, r1] = _, r0 = +r0, r1 = +r1, round = true, rescale();
  };
  scale2.bandwidth = function() {
    return bandwidth;
  };
  scale2.step = function() {
    return step;
  };
  scale2.round = function(_) {
    return arguments.length ? (round = !!_, rescale()) : round;
  };
  scale2.padding = function(_) {
    return arguments.length ? (paddingInner = Math.min(1, paddingOuter = +_), rescale()) : paddingInner;
  };
  scale2.paddingInner = function(_) {
    return arguments.length ? (paddingInner = Math.min(1, _), rescale()) : paddingInner;
  };
  scale2.paddingOuter = function(_) {
    return arguments.length ? (paddingOuter = +_, rescale()) : paddingOuter;
  };
  scale2.align = function(_) {
    return arguments.length ? (align = Math.max(0, Math.min(1, _)), rescale()) : align;
  };
  scale2.copy = function() {
    return band(domain(), [r0, r1]).round(round).paddingInner(paddingInner).paddingOuter(paddingOuter).align(align);
  };
  return initRange.apply(rescale(), arguments);
}
function pointish(scale2) {
  var copy = scale2.copy;
  scale2.padding = scale2.paddingOuter;
  delete scale2.paddingInner;
  delete scale2.paddingOuter;
  scale2.copy = function() {
    return pointish(copy());
  };
  return scale2;
}
function point() {
  return pointish(band.apply(null, arguments).paddingInner(1));
}
// node_modules/d3-scale/src/constant.js
function constants(x3) {
  return function() {
    return x3;
  };
}

// node_modules/d3-scale/src/number.js
function number3(x3) {
  return +x3;
}

// node_modules/d3-scale/src/continuous.js
var unit = [0, 1];
function identity3(x3) {
  return x3;
}
function normalize2(a3, b) {
  return (b -= a3 = +a3) ? function(x3) {
    return (x3 - a3) / b;
  } : constants(isNaN(b) ? NaN : 0.5);
}
function clamper(a3, b) {
  var t;
  if (a3 > b)
    t = a3, a3 = b, b = t;
  return function(x3) {
    return Math.max(a3, Math.min(b, x3));
  };
}
function bimap(domain, range4, interpolate) {
  var d0 = domain[0], d1 = domain[1], r0 = range4[0], r1 = range4[1];
  if (d1 < d0)
    d0 = normalize2(d1, d0), r0 = interpolate(r1, r0);
  else
    d0 = normalize2(d0, d1), r0 = interpolate(r0, r1);
  return function(x3) {
    return r0(d0(x3));
  };
}
function polymap(domain, range4, interpolate) {
  var j = Math.min(domain.length, range4.length) - 1, d = new Array(j), r = new Array(j), i = -1;
  if (domain[j] < domain[0]) {
    domain = domain.slice().reverse();
    range4 = range4.slice().reverse();
  }
  while (++i < j) {
    d[i] = normalize2(domain[i], domain[i + 1]);
    r[i] = interpolate(range4[i], range4[i + 1]);
  }
  return function(x3) {
    var i2 = bisect_default(domain, x3, 1, j) - 1;
    return r[i2](d[i2](x3));
  };
}
function copy(source, target) {
  return target.domain(source.domain()).range(source.range()).interpolate(source.interpolate()).clamp(source.clamp()).unknown(source.unknown());
}
function transformer2() {
  var domain = unit, range4 = unit, interpolate = value_default, transform, untransform, unknown, clamp = identity3, piecewise2, output, input;
  function rescale() {
    var n = Math.min(domain.length, range4.length);
    if (clamp !== identity3)
      clamp = clamper(domain[0], domain[n - 1]);
    piecewise2 = n > 2 ? polymap : bimap;
    output = input = null;
    return scale2;
  }
  function scale2(x3) {
    return x3 == null || isNaN(x3 = +x3) ? unknown : (output || (output = piecewise2(domain.map(transform), range4, interpolate)))(transform(clamp(x3)));
  }
  scale2.invert = function(y3) {
    return clamp(untransform((input || (input = piecewise2(range4, domain.map(transform), number_default)))(y3)));
  };
  scale2.domain = function(_) {
    return arguments.length ? (domain = Array.from(_, number3), rescale()) : domain.slice();
  };
  scale2.range = function(_) {
    return arguments.length ? (range4 = Array.from(_), rescale()) : range4.slice();
  };
  scale2.rangeRound = function(_) {
    return range4 = Array.from(_), interpolate = round_default, rescale();
  };
  scale2.clamp = function(_) {
    return arguments.length ? (clamp = _ ? true : identity3, rescale()) : clamp !== identity3;
  };
  scale2.interpolate = function(_) {
    return arguments.length ? (interpolate = _, rescale()) : interpolate;
  };
  scale2.unknown = function(_) {
    return arguments.length ? (unknown = _, scale2) : unknown;
  };
  return function(t, u4) {
    transform = t, untransform = u4;
    return rescale();
  };
}
function continuous() {
  return transformer2()(identity3, identity3);
}

// node_modules/d3-scale/src/tickFormat.js
function tickFormat(start2, stop, count3, specifier) {
  var step = tickStep(start2, stop, count3), precision;
  specifier = formatSpecifier(specifier == null ? ",f" : specifier);
  switch (specifier.type) {
    case "s": {
      var value = Math.max(Math.abs(start2), Math.abs(stop));
      if (specifier.precision == null && !isNaN(precision = precisionPrefix_default(step, value)))
        specifier.precision = precision;
      return formatPrefix(specifier, value);
    }
    case "":
    case "e":
    case "g":
    case "p":
    case "r": {
      if (specifier.precision == null && !isNaN(precision = precisionRound_default(step, Math.max(Math.abs(start2), Math.abs(stop)))))
        specifier.precision = precision - (specifier.type === "e");
      break;
    }
    case "f":
    case "%": {
      if (specifier.precision == null && !isNaN(precision = precisionFixed_default(step)))
        specifier.precision = precision - (specifier.type === "%") * 2;
      break;
    }
  }
  return format(specifier);
}

// node_modules/d3-scale/src/linear.js
function linearish(scale2) {
  var domain = scale2.domain;
  scale2.ticks = function(count3) {
    var d = domain();
    return ticks(d[0], d[d.length - 1], count3 == null ? 10 : count3);
  };
  scale2.tickFormat = function(count3, specifier) {
    var d = domain();
    return tickFormat(d[0], d[d.length - 1], count3 == null ? 10 : count3, specifier);
  };
  scale2.nice = function(count3) {
    if (count3 == null)
      count3 = 10;
    var d = domain();
    var i0 = 0;
    var i1 = d.length - 1;
    var start2 = d[i0];
    var stop = d[i1];
    var prestep;
    var step;
    var maxIter = 10;
    if (stop < start2) {
      step = start2, start2 = stop, stop = step;
      step = i0, i0 = i1, i1 = step;
    }
    while (maxIter-- > 0) {
      step = tickIncrement(start2, stop, count3);
      if (step === prestep) {
        d[i0] = start2;
        d[i1] = stop;
        return domain(d);
      } else if (step > 0) {
        start2 = Math.floor(start2 / step) * step;
        stop = Math.ceil(stop / step) * step;
      } else if (step < 0) {
        start2 = Math.ceil(start2 * step) / step;
        stop = Math.floor(stop * step) / step;
      } else {
        break;
      }
      prestep = step;
    }
    return scale2;
  };
  return scale2;
}
function linear3() {
  var scale2 = continuous();
  scale2.copy = function() {
    return copy(scale2, linear3());
  };
  initRange.apply(scale2, arguments);
  return linearish(scale2);
}

// node_modules/d3-scale/src/identity.js
function identity4(domain) {
  var unknown;
  function scale2(x3) {
    return x3 == null || isNaN(x3 = +x3) ? unknown : x3;
  }
  scale2.invert = scale2;
  scale2.domain = scale2.range = function(_) {
    return arguments.length ? (domain = Array.from(_, number3), scale2) : domain.slice();
  };
  scale2.unknown = function(_) {
    return arguments.length ? (unknown = _, scale2) : unknown;
  };
  scale2.copy = function() {
    return identity4(domain).unknown(unknown);
  };
  domain = arguments.length ? Array.from(domain, number3) : [0, 1];
  return linearish(scale2);
}
// node_modules/d3-scale/src/nice.js
function nice2(domain, interval2) {
  domain = domain.slice();
  var i0 = 0, i1 = domain.length - 1, x06 = domain[i0], x12 = domain[i1], t;
  if (x12 < x06) {
    t = i0, i0 = i1, i1 = t;
    t = x06, x06 = x12, x12 = t;
  }
  domain[i0] = interval2.floor(x06);
  domain[i1] = interval2.ceil(x12);
  return domain;
}

// node_modules/d3-scale/src/log.js
function transformLog(x3) {
  return Math.log(x3);
}
function transformExp(x3) {
  return Math.exp(x3);
}
function transformLogn(x3) {
  return -Math.log(-x3);
}
function transformExpn(x3) {
  return -Math.exp(-x3);
}
function pow10(x3) {
  return isFinite(x3) ? +("1e" + x3) : x3 < 0 ? 0 : x3;
}
function powp(base) {
  return base === 10 ? pow10 : base === Math.E ? Math.exp : (x3) => Math.pow(base, x3);
}
function logp(base) {
  return base === Math.E ? Math.log : base === 10 && Math.log10 || base === 2 && Math.log2 || (base = Math.log(base), (x3) => Math.log(x3) / base);
}
function reflect(f) {
  return (x3, k) => -f(-x3, k);
}
function loggish(transform) {
  const scale2 = transform(transformLog, transformExp);
  const domain = scale2.domain;
  let base = 10;
  let logs;
  let pows;
  function rescale() {
    logs = logp(base), pows = powp(base);
    if (domain()[0] < 0) {
      logs = reflect(logs), pows = reflect(pows);
      transform(transformLogn, transformExpn);
    } else {
      transform(transformLog, transformExp);
    }
    return scale2;
  }
  scale2.base = function(_) {
    return arguments.length ? (base = +_, rescale()) : base;
  };
  scale2.domain = function(_) {
    return arguments.length ? (domain(_), rescale()) : domain();
  };
  scale2.ticks = (count3) => {
    const d = domain();
    let u4 = d[0];
    let v2 = d[d.length - 1];
    const r = v2 < u4;
    if (r)
      [u4, v2] = [v2, u4];
    let i = logs(u4);
    let j = logs(v2);
    let k;
    let t;
    const n = count3 == null ? 10 : +count3;
    let z = [];
    if (!(base % 1) && j - i < n) {
      i = Math.floor(i), j = Math.ceil(j);
      if (u4 > 0)
        for (;i <= j; ++i) {
          for (k = 1;k < base; ++k) {
            t = i < 0 ? k / pows(-i) : k * pows(i);
            if (t < u4)
              continue;
            if (t > v2)
              break;
            z.push(t);
          }
        }
      else
        for (;i <= j; ++i) {
          for (k = base - 1;k >= 1; --k) {
            t = i > 0 ? k / pows(-i) : k * pows(i);
            if (t < u4)
              continue;
            if (t > v2)
              break;
            z.push(t);
          }
        }
      if (z.length * 2 < n)
        z = ticks(u4, v2, n);
    } else {
      z = ticks(i, j, Math.min(j - i, n)).map(pows);
    }
    return r ? z.reverse() : z;
  };
  scale2.tickFormat = (count3, specifier) => {
    if (count3 == null)
      count3 = 10;
    if (specifier == null)
      specifier = base === 10 ? "s" : ",";
    if (typeof specifier !== "function") {
      if (!(base % 1) && (specifier = formatSpecifier(specifier)).precision == null)
        specifier.trim = true;
      specifier = format(specifier);
    }
    if (count3 === Infinity)
      return specifier;
    const k = Math.max(1, base * count3 / scale2.ticks().length);
    return (d) => {
      let i = d / pows(Math.round(logs(d)));
      if (i * base < base - 0.5)
        i *= base;
      return i <= k ? specifier(d) : "";
    };
  };
  scale2.nice = () => {
    return domain(nice2(domain(), {
      floor: (x3) => pows(Math.floor(logs(x3))),
      ceil: (x3) => pows(Math.ceil(logs(x3)))
    }));
  };
  return scale2;
}
function log2() {
  const scale2 = loggish(transformer2()).domain([1, 10]);
  scale2.copy = () => copy(scale2, log2()).base(scale2.base());
  initRange.apply(scale2, arguments);
  return scale2;
}
// node_modules/d3-scale/src/symlog.js
function transformSymlog(c3) {
  return function(x3) {
    return Math.sign(x3) * Math.log1p(Math.abs(x3 / c3));
  };
}
function transformSymexp(c3) {
  return function(x3) {
    return Math.sign(x3) * Math.expm1(Math.abs(x3)) * c3;
  };
}
function symlogish(transform) {
  var c3 = 1, scale2 = transform(transformSymlog(c3), transformSymexp(c3));
  scale2.constant = function(_) {
    return arguments.length ? transform(transformSymlog(c3 = +_), transformSymexp(c3)) : c3;
  };
  return linearish(scale2);
}
function symlog() {
  var scale2 = symlogish(transformer2());
  scale2.copy = function() {
    return copy(scale2, symlog()).constant(scale2.constant());
  };
  return initRange.apply(scale2, arguments);
}
// node_modules/d3-scale/src/pow.js
function transformPow(exponent2) {
  return function(x3) {
    return x3 < 0 ? -Math.pow(-x3, exponent2) : Math.pow(x3, exponent2);
  };
}
function transformSqrt(x3) {
  return x3 < 0 ? -Math.sqrt(-x3) : Math.sqrt(x3);
}
function transformSquare(x3) {
  return x3 < 0 ? -x3 * x3 : x3 * x3;
}
function powish(transform) {
  var scale2 = transform(identity3, identity3), exponent2 = 1;
  function rescale() {
    return exponent2 === 1 ? transform(identity3, identity3) : exponent2 === 0.5 ? transform(transformSqrt, transformSquare) : transform(transformPow(exponent2), transformPow(1 / exponent2));
  }
  scale2.exponent = function(_) {
    return arguments.length ? (exponent2 = +_, rescale()) : exponent2;
  };
  return linearish(scale2);
}
function pow3() {
  var scale2 = powish(transformer2());
  scale2.copy = function() {
    return copy(scale2, pow3()).exponent(scale2.exponent());
  };
  initRange.apply(scale2, arguments);
  return scale2;
}
function sqrt2() {
  return pow3.apply(null, arguments).exponent(0.5);
}
// node_modules/d3-scale/src/radial.js
function square(x3) {
  return Math.sign(x3) * x3 * x3;
}
function unsquare(x3) {
  return Math.sign(x3) * Math.sqrt(Math.abs(x3));
}
function radial() {
  var squared = continuous(), range4 = [0, 1], round = false, unknown;
  function scale2(x3) {
    var y3 = unsquare(squared(x3));
    return isNaN(y3) ? unknown : round ? Math.round(y3) : y3;
  }
  scale2.invert = function(y3) {
    return squared.invert(square(y3));
  };
  scale2.domain = function(_) {
    return arguments.length ? (squared.domain(_), scale2) : squared.domain();
  };
  scale2.range = function(_) {
    return arguments.length ? (squared.range((range4 = Array.from(_, number3)).map(square)), scale2) : range4.slice();
  };
  scale2.rangeRound = function(_) {
    return scale2.range(_).round(true);
  };
  scale2.round = function(_) {
    return arguments.length ? (round = !!_, scale2) : round;
  };
  scale2.clamp = function(_) {
    return arguments.length ? (squared.clamp(_), scale2) : squared.clamp();
  };
  scale2.unknown = function(_) {
    return arguments.length ? (unknown = _, scale2) : unknown;
  };
  scale2.copy = function() {
    return radial(squared.domain(), range4).round(round).clamp(squared.clamp()).unknown(unknown);
  };
  initRange.apply(scale2, arguments);
  return linearish(scale2);
}
// node_modules/d3-scale/src/quantile.js
function quantile2() {
  var domain = [], range4 = [], thresholds = [], unknown;
  function rescale() {
    var i = 0, n = Math.max(1, range4.length);
    thresholds = new Array(n - 1);
    while (++i < n)
      thresholds[i - 1] = quantileSorted(domain, i / n);
    return scale2;
  }
  function scale2(x3) {
    return x3 == null || isNaN(x3 = +x3) ? unknown : range4[bisect_default(thresholds, x3)];
  }
  scale2.invertExtent = function(y3) {
    var i = range4.indexOf(y3);
    return i < 0 ? [NaN, NaN] : [
      i > 0 ? thresholds[i - 1] : domain[0],
      i < thresholds.length ? thresholds[i] : domain[domain.length - 1]
    ];
  };
  scale2.domain = function(_) {
    if (!arguments.length)
      return domain.slice();
    domain = [];
    for (let d of _)
      if (d != null && !isNaN(d = +d))
        domain.push(d);
    domain.sort(ascending);
    return rescale();
  };
  scale2.range = function(_) {
    return arguments.length ? (range4 = Array.from(_), rescale()) : range4.slice();
  };
  scale2.unknown = function(_) {
    return arguments.length ? (unknown = _, scale2) : unknown;
  };
  scale2.quantiles = function() {
    return thresholds.slice();
  };
  scale2.copy = function() {
    return quantile2().domain(domain).range(range4).unknown(unknown);
  };
  return initRange.apply(scale2, arguments);
}
// node_modules/d3-scale/src/quantize.js
function quantize() {
  var x06 = 0, x12 = 1, n = 1, domain = [0.5], range4 = [0, 1], unknown;
  function scale2(x3) {
    return x3 != null && x3 <= x3 ? range4[bisect_default(domain, x3, 0, n)] : unknown;
  }
  function rescale() {
    var i = -1;
    domain = new Array(n);
    while (++i < n)
      domain[i] = ((i + 1) * x12 - (i - n) * x06) / (n + 1);
    return scale2;
  }
  scale2.domain = function(_) {
    return arguments.length ? ([x06, x12] = _, x06 = +x06, x12 = +x12, rescale()) : [x06, x12];
  };
  scale2.range = function(_) {
    return arguments.length ? (n = (range4 = Array.from(_)).length - 1, rescale()) : range4.slice();
  };
  scale2.invertExtent = function(y3) {
    var i = range4.indexOf(y3);
    return i < 0 ? [NaN, NaN] : i < 1 ? [x06, domain[0]] : i >= n ? [domain[n - 1], x12] : [domain[i - 1], domain[i]];
  };
  scale2.unknown = function(_) {
    return arguments.length ? (unknown = _, scale2) : scale2;
  };
  scale2.thresholds = function() {
    return domain.slice();
  };
  scale2.copy = function() {
    return quantize().domain([x06, x12]).range(range4).unknown(unknown);
  };
  return initRange.apply(linearish(scale2), arguments);
}
// node_modules/d3-scale/src/threshold.js
function threshold() {
  var domain = [0.5], range4 = [0, 1], unknown, n = 1;
  function scale2(x3) {
    return x3 != null && x3 <= x3 ? range4[bisect_default(domain, x3, 0, n)] : unknown;
  }
  scale2.domain = function(_) {
    return arguments.length ? (domain = Array.from(_), n = Math.min(domain.length, range4.length - 1), scale2) : domain.slice();
  };
  scale2.range = function(_) {
    return arguments.length ? (range4 = Array.from(_), n = Math.min(domain.length, range4.length - 1), scale2) : range4.slice();
  };
  scale2.invertExtent = function(y3) {
    var i = range4.indexOf(y3);
    return [domain[i - 1], domain[i]];
  };
  scale2.unknown = function(_) {
    return arguments.length ? (unknown = _, scale2) : unknown;
  };
  scale2.copy = function() {
    return threshold().domain(domain).range(range4).unknown(unknown);
  };
  return initRange.apply(scale2, arguments);
}
// node_modules/d3-time/src/interval.js
var t02 = new Date;
var t12 = new Date;
function timeInterval(floori, offseti, count3, field) {
  function interval2(date) {
    return floori(date = arguments.length === 0 ? new Date : new Date(+date)), date;
  }
  interval2.floor = (date) => {
    return floori(date = new Date(+date)), date;
  };
  interval2.ceil = (date) => {
    return floori(date = new Date(date - 1)), offseti(date, 1), floori(date), date;
  };
  interval2.round = (date) => {
    const d0 = interval2(date), d1 = interval2.ceil(date);
    return date - d0 < d1 - date ? d0 : d1;
  };
  interval2.offset = (date, step) => {
    return offseti(date = new Date(+date), step == null ? 1 : Math.floor(step)), date;
  };
  interval2.range = (start2, stop, step) => {
    const range4 = [];
    start2 = interval2.ceil(start2);
    step = step == null ? 1 : Math.floor(step);
    if (!(start2 < stop) || !(step > 0))
      return range4;
    let previous;
    do
      range4.push(previous = new Date(+start2)), offseti(start2, step), floori(start2);
    while (previous < start2 && start2 < stop);
    return range4;
  };
  interval2.filter = (test) => {
    return timeInterval((date) => {
      if (date >= date)
        while (floori(date), !test(date))
          date.setTime(date - 1);
    }, (date, step) => {
      if (date >= date) {
        if (step < 0)
          while (++step <= 0) {
            while (offseti(date, -1), !test(date)) {}
          }
        else
          while (--step >= 0) {
            while (offseti(date, 1), !test(date)) {}
          }
      }
    });
  };
  if (count3) {
    interval2.count = (start2, end) => {
      t02.setTime(+start2), t12.setTime(+end);
      floori(t02), floori(t12);
      return Math.floor(count3(t02, t12));
    };
    interval2.every = (step) => {
      step = Math.floor(step);
      return !isFinite(step) || !(step > 0) ? null : !(step > 1) ? interval2 : interval2.filter(field ? (d) => field(d) % step === 0 : (d) => interval2.count(0, d) % step === 0);
    };
  }
  return interval2;
}
// node_modules/d3-time/src/millisecond.js
var millisecond = timeInterval(() => {}, (date, step) => {
  date.setTime(+date + step);
}, (start2, end) => {
  return end - start2;
});
millisecond.every = (k) => {
  k = Math.floor(k);
  if (!isFinite(k) || !(k > 0))
    return null;
  if (!(k > 1))
    return millisecond;
  return timeInterval((date) => {
    date.setTime(Math.floor(date / k) * k);
  }, (date, step) => {
    date.setTime(+date + step * k);
  }, (start2, end) => {
    return (end - start2) / k;
  });
};
var milliseconds = millisecond.range;
// node_modules/d3-time/src/duration.js
var durationSecond = 1000;
var durationMinute = durationSecond * 60;
var durationHour = durationMinute * 60;
var durationDay = durationHour * 24;
var durationWeek = durationDay * 7;
var durationMonth = durationDay * 30;
var durationYear = durationDay * 365;

// node_modules/d3-time/src/second.js
var second = timeInterval((date) => {
  date.setTime(date - date.getMilliseconds());
}, (date, step) => {
  date.setTime(+date + step * durationSecond);
}, (start2, end) => {
  return (end - start2) / durationSecond;
}, (date) => {
  return date.getUTCSeconds();
});
var seconds = second.range;
// node_modules/d3-time/src/minute.js
var timeMinute = timeInterval((date) => {
  date.setTime(date - date.getMilliseconds() - date.getSeconds() * durationSecond);
}, (date, step) => {
  date.setTime(+date + step * durationMinute);
}, (start2, end) => {
  return (end - start2) / durationMinute;
}, (date) => {
  return date.getMinutes();
});
var timeMinutes = timeMinute.range;
var utcMinute = timeInterval((date) => {
  date.setUTCSeconds(0, 0);
}, (date, step) => {
  date.setTime(+date + step * durationMinute);
}, (start2, end) => {
  return (end - start2) / durationMinute;
}, (date) => {
  return date.getUTCMinutes();
});
var utcMinutes = utcMinute.range;
// node_modules/d3-time/src/hour.js
var timeHour = timeInterval((date) => {
  date.setTime(date - date.getMilliseconds() - date.getSeconds() * durationSecond - date.getMinutes() * durationMinute);
}, (date, step) => {
  date.setTime(+date + step * durationHour);
}, (start2, end) => {
  return (end - start2) / durationHour;
}, (date) => {
  return date.getHours();
});
var timeHours = timeHour.range;
var utcHour = timeInterval((date) => {
  date.setUTCMinutes(0, 0, 0);
}, (date, step) => {
  date.setTime(+date + step * durationHour);
}, (start2, end) => {
  return (end - start2) / durationHour;
}, (date) => {
  return date.getUTCHours();
});
var utcHours = utcHour.range;
// node_modules/d3-time/src/day.js
var timeDay = timeInterval((date) => date.setHours(0, 0, 0, 0), (date, step) => date.setDate(date.getDate() + step), (start2, end) => (end - start2 - (end.getTimezoneOffset() - start2.getTimezoneOffset()) * durationMinute) / durationDay, (date) => date.getDate() - 1);
var timeDays = timeDay.range;
var utcDay = timeInterval((date) => {
  date.setUTCHours(0, 0, 0, 0);
}, (date, step) => {
  date.setUTCDate(date.getUTCDate() + step);
}, (start2, end) => {
  return (end - start2) / durationDay;
}, (date) => {
  return date.getUTCDate() - 1;
});
var utcDays = utcDay.range;
var unixDay = timeInterval((date) => {
  date.setUTCHours(0, 0, 0, 0);
}, (date, step) => {
  date.setUTCDate(date.getUTCDate() + step);
}, (start2, end) => {
  return (end - start2) / durationDay;
}, (date) => {
  return Math.floor(date / durationDay);
});
var unixDays = unixDay.range;
// node_modules/d3-time/src/week.js
function timeWeekday(i) {
  return timeInterval((date) => {
    date.setDate(date.getDate() - (date.getDay() + 7 - i) % 7);
    date.setHours(0, 0, 0, 0);
  }, (date, step) => {
    date.setDate(date.getDate() + step * 7);
  }, (start2, end) => {
    return (end - start2 - (end.getTimezoneOffset() - start2.getTimezoneOffset()) * durationMinute) / durationWeek;
  });
}
var timeSunday = timeWeekday(0);
var timeMonday = timeWeekday(1);
var timeTuesday = timeWeekday(2);
var timeWednesday = timeWeekday(3);
var timeThursday = timeWeekday(4);
var timeFriday = timeWeekday(5);
var timeSaturday = timeWeekday(6);
var timeSundays = timeSunday.range;
var timeMondays = timeMonday.range;
var timeTuesdays = timeTuesday.range;
var timeWednesdays = timeWednesday.range;
var timeThursdays = timeThursday.range;
var timeFridays = timeFriday.range;
var timeSaturdays = timeSaturday.range;
function utcWeekday(i) {
  return timeInterval((date) => {
    date.setUTCDate(date.getUTCDate() - (date.getUTCDay() + 7 - i) % 7);
    date.setUTCHours(0, 0, 0, 0);
  }, (date, step) => {
    date.setUTCDate(date.getUTCDate() + step * 7);
  }, (start2, end) => {
    return (end - start2) / durationWeek;
  });
}
var utcSunday = utcWeekday(0);
var utcMonday = utcWeekday(1);
var utcTuesday = utcWeekday(2);
var utcWednesday = utcWeekday(3);
var utcThursday = utcWeekday(4);
var utcFriday = utcWeekday(5);
var utcSaturday = utcWeekday(6);
var utcSundays = utcSunday.range;
var utcMondays = utcMonday.range;
var utcTuesdays = utcTuesday.range;
var utcWednesdays = utcWednesday.range;
var utcThursdays = utcThursday.range;
var utcFridays = utcFriday.range;
var utcSaturdays = utcSaturday.range;
// node_modules/d3-time/src/month.js
var timeMonth = timeInterval((date) => {
  date.setDate(1);
  date.setHours(0, 0, 0, 0);
}, (date, step) => {
  date.setMonth(date.getMonth() + step);
}, (start2, end) => {
  return end.getMonth() - start2.getMonth() + (end.getFullYear() - start2.getFullYear()) * 12;
}, (date) => {
  return date.getMonth();
});
var timeMonths = timeMonth.range;
var utcMonth = timeInterval((date) => {
  date.setUTCDate(1);
  date.setUTCHours(0, 0, 0, 0);
}, (date, step) => {
  date.setUTCMonth(date.getUTCMonth() + step);
}, (start2, end) => {
  return end.getUTCMonth() - start2.getUTCMonth() + (end.getUTCFullYear() - start2.getUTCFullYear()) * 12;
}, (date) => {
  return date.getUTCMonth();
});
var utcMonths = utcMonth.range;
// node_modules/d3-time/src/year.js
var timeYear = timeInterval((date) => {
  date.setMonth(0, 1);
  date.setHours(0, 0, 0, 0);
}, (date, step) => {
  date.setFullYear(date.getFullYear() + step);
}, (start2, end) => {
  return end.getFullYear() - start2.getFullYear();
}, (date) => {
  return date.getFullYear();
});
timeYear.every = (k) => {
  return !isFinite(k = Math.floor(k)) || !(k > 0) ? null : timeInterval((date) => {
    date.setFullYear(Math.floor(date.getFullYear() / k) * k);
    date.setMonth(0, 1);
    date.setHours(0, 0, 0, 0);
  }, (date, step) => {
    date.setFullYear(date.getFullYear() + step * k);
  });
};
var timeYears = timeYear.range;
var utcYear = timeInterval((date) => {
  date.setUTCMonth(0, 1);
  date.setUTCHours(0, 0, 0, 0);
}, (date, step) => {
  date.setUTCFullYear(date.getUTCFullYear() + step);
}, (start2, end) => {
  return end.getUTCFullYear() - start2.getUTCFullYear();
}, (date) => {
  return date.getUTCFullYear();
});
utcYear.every = (k) => {
  return !isFinite(k = Math.floor(k)) || !(k > 0) ? null : timeInterval((date) => {
    date.setUTCFullYear(Math.floor(date.getUTCFullYear() / k) * k);
    date.setUTCMonth(0, 1);
    date.setUTCHours(0, 0, 0, 0);
  }, (date, step) => {
    date.setUTCFullYear(date.getUTCFullYear() + step * k);
  });
};
var utcYears = utcYear.range;
// node_modules/d3-time/src/ticks.js
function ticker(year, month, week, day, hour, minute) {
  const tickIntervals = [
    [second, 1, durationSecond],
    [second, 5, 5 * durationSecond],
    [second, 15, 15 * durationSecond],
    [second, 30, 30 * durationSecond],
    [minute, 1, durationMinute],
    [minute, 5, 5 * durationMinute],
    [minute, 15, 15 * durationMinute],
    [minute, 30, 30 * durationMinute],
    [hour, 1, durationHour],
    [hour, 3, 3 * durationHour],
    [hour, 6, 6 * durationHour],
    [hour, 12, 12 * durationHour],
    [day, 1, durationDay],
    [day, 2, 2 * durationDay],
    [week, 1, durationWeek],
    [month, 1, durationMonth],
    [month, 3, 3 * durationMonth],
    [year, 1, durationYear]
  ];
  function ticks2(start2, stop, count3) {
    const reverse2 = stop < start2;
    if (reverse2)
      [start2, stop] = [stop, start2];
    const interval2 = count3 && typeof count3.range === "function" ? count3 : tickInterval(start2, stop, count3);
    const ticks3 = interval2 ? interval2.range(start2, +stop + 1) : [];
    return reverse2 ? ticks3.reverse() : ticks3;
  }
  function tickInterval(start2, stop, count3) {
    const target = Math.abs(stop - start2) / count3;
    const i = bisector(([, , step2]) => step2).right(tickIntervals, target);
    if (i === tickIntervals.length)
      return year.every(tickStep(start2 / durationYear, stop / durationYear, count3));
    if (i === 0)
      return millisecond.every(Math.max(tickStep(start2, stop, count3), 1));
    const [t, step] = tickIntervals[target / tickIntervals[i - 1][2] < tickIntervals[i][2] / target ? i - 1 : i];
    return t.every(step);
  }
  return [ticks2, tickInterval];
}
var [utcTicks, utcTickInterval] = ticker(utcYear, utcMonth, utcSunday, unixDay, utcHour, utcMinute);
var [timeTicks, timeTickInterval] = ticker(timeYear, timeMonth, timeSunday, timeDay, timeHour, timeMinute);
// node_modules/d3-time-format/src/locale.js
function localDate(d) {
  if (0 <= d.y && d.y < 100) {
    var date = new Date(-1, d.m, d.d, d.H, d.M, d.S, d.L);
    date.setFullYear(d.y);
    return date;
  }
  return new Date(d.y, d.m, d.d, d.H, d.M, d.S, d.L);
}
function utcDate(d) {
  if (0 <= d.y && d.y < 100) {
    var date = new Date(Date.UTC(-1, d.m, d.d, d.H, d.M, d.S, d.L));
    date.setUTCFullYear(d.y);
    return date;
  }
  return new Date(Date.UTC(d.y, d.m, d.d, d.H, d.M, d.S, d.L));
}
function newDate(y3, m3, d) {
  return { y: y3, m: m3, d, H: 0, M: 0, S: 0, L: 0 };
}
function formatLocale(locale2) {
  var { dateTime: locale_dateTime, date: locale_date, time: locale_time, periods: locale_periods, days: locale_weekdays, shortDays: locale_shortWeekdays, months: locale_months, shortMonths: locale_shortMonths } = locale2;
  var periodRe = formatRe(locale_periods), periodLookup = formatLookup(locale_periods), weekdayRe = formatRe(locale_weekdays), weekdayLookup = formatLookup(locale_weekdays), shortWeekdayRe = formatRe(locale_shortWeekdays), shortWeekdayLookup = formatLookup(locale_shortWeekdays), monthRe = formatRe(locale_months), monthLookup = formatLookup(locale_months), shortMonthRe = formatRe(locale_shortMonths), shortMonthLookup = formatLookup(locale_shortMonths);
  var formats = {
    a: formatShortWeekday,
    A: formatWeekday,
    b: formatShortMonth,
    B: formatMonth,
    c: null,
    d: formatDayOfMonth,
    e: formatDayOfMonth,
    f: formatMicroseconds,
    g: formatYearISO,
    G: formatFullYearISO,
    H: formatHour24,
    I: formatHour12,
    j: formatDayOfYear,
    L: formatMilliseconds,
    m: formatMonthNumber,
    M: formatMinutes,
    p: formatPeriod,
    q: formatQuarter,
    Q: formatUnixTimestamp,
    s: formatUnixTimestampSeconds,
    S: formatSeconds,
    u: formatWeekdayNumberMonday,
    U: formatWeekNumberSunday,
    V: formatWeekNumberISO,
    w: formatWeekdayNumberSunday,
    W: formatWeekNumberMonday,
    x: null,
    X: null,
    y: formatYear2,
    Y: formatFullYear,
    Z: formatZone,
    "%": formatLiteralPercent
  };
  var utcFormats = {
    a: formatUTCShortWeekday,
    A: formatUTCWeekday,
    b: formatUTCShortMonth,
    B: formatUTCMonth,
    c: null,
    d: formatUTCDayOfMonth,
    e: formatUTCDayOfMonth,
    f: formatUTCMicroseconds,
    g: formatUTCYearISO,
    G: formatUTCFullYearISO,
    H: formatUTCHour24,
    I: formatUTCHour12,
    j: formatUTCDayOfYear,
    L: formatUTCMilliseconds,
    m: formatUTCMonthNumber,
    M: formatUTCMinutes,
    p: formatUTCPeriod,
    q: formatUTCQuarter,
    Q: formatUnixTimestamp,
    s: formatUnixTimestampSeconds,
    S: formatUTCSeconds,
    u: formatUTCWeekdayNumberMonday,
    U: formatUTCWeekNumberSunday,
    V: formatUTCWeekNumberISO,
    w: formatUTCWeekdayNumberSunday,
    W: formatUTCWeekNumberMonday,
    x: null,
    X: null,
    y: formatUTCYear,
    Y: formatUTCFullYear,
    Z: formatUTCZone,
    "%": formatLiteralPercent
  };
  var parses = {
    a: parseShortWeekday,
    A: parseWeekday,
    b: parseShortMonth,
    B: parseMonth,
    c: parseLocaleDateTime,
    d: parseDayOfMonth,
    e: parseDayOfMonth,
    f: parseMicroseconds,
    g: parseYear,
    G: parseFullYear,
    H: parseHour24,
    I: parseHour24,
    j: parseDayOfYear,
    L: parseMilliseconds,
    m: parseMonthNumber,
    M: parseMinutes,
    p: parsePeriod,
    q: parseQuarter,
    Q: parseUnixTimestamp,
    s: parseUnixTimestampSeconds,
    S: parseSeconds,
    u: parseWeekdayNumberMonday,
    U: parseWeekNumberSunday,
    V: parseWeekNumberISO,
    w: parseWeekdayNumberSunday,
    W: parseWeekNumberMonday,
    x: parseLocaleDate,
    X: parseLocaleTime,
    y: parseYear,
    Y: parseFullYear,
    Z: parseZone,
    "%": parseLiteralPercent
  };
  formats.x = newFormat(locale_date, formats);
  formats.X = newFormat(locale_time, formats);
  formats.c = newFormat(locale_dateTime, formats);
  utcFormats.x = newFormat(locale_date, utcFormats);
  utcFormats.X = newFormat(locale_time, utcFormats);
  utcFormats.c = newFormat(locale_dateTime, utcFormats);
  function newFormat(specifier, formats2) {
    return function(date) {
      var string = [], i = -1, j = 0, n = specifier.length, c3, pad2, format2;
      if (!(date instanceof Date))
        date = new Date(+date);
      while (++i < n) {
        if (specifier.charCodeAt(i) === 37) {
          string.push(specifier.slice(j, i));
          if ((pad2 = pads[c3 = specifier.charAt(++i)]) != null)
            c3 = specifier.charAt(++i);
          else
            pad2 = c3 === "e" ? " " : "0";
          if (format2 = formats2[c3])
            c3 = format2(date, pad2);
          string.push(c3);
          j = i + 1;
        }
      }
      string.push(specifier.slice(j, i));
      return string.join("");
    };
  }
  function newParse(specifier, Z) {
    return function(string) {
      var d = newDate(1900, undefined, 1), i = parseSpecifier(d, specifier, string += "", 0), week, day;
      if (i != string.length)
        return null;
      if ("Q" in d)
        return new Date(d.Q);
      if ("s" in d)
        return new Date(d.s * 1000 + ("L" in d ? d.L : 0));
      if (Z && !("Z" in d))
        d.Z = 0;
      if ("p" in d)
        d.H = d.H % 12 + d.p * 12;
      if (d.m === undefined)
        d.m = "q" in d ? d.q : 0;
      if ("V" in d) {
        if (d.V < 1 || d.V > 53)
          return null;
        if (!("w" in d))
          d.w = 1;
        if ("Z" in d) {
          week = utcDate(newDate(d.y, 0, 1)), day = week.getUTCDay();
          week = day > 4 || day === 0 ? utcMonday.ceil(week) : utcMonday(week);
          week = utcDay.offset(week, (d.V - 1) * 7);
          d.y = week.getUTCFullYear();
          d.m = week.getUTCMonth();
          d.d = week.getUTCDate() + (d.w + 6) % 7;
        } else {
          week = localDate(newDate(d.y, 0, 1)), day = week.getDay();
          week = day > 4 || day === 0 ? timeMonday.ceil(week) : timeMonday(week);
          week = timeDay.offset(week, (d.V - 1) * 7);
          d.y = week.getFullYear();
          d.m = week.getMonth();
          d.d = week.getDate() + (d.w + 6) % 7;
        }
      } else if ("W" in d || "U" in d) {
        if (!("w" in d))
          d.w = "u" in d ? d.u % 7 : ("W" in d) ? 1 : 0;
        day = "Z" in d ? utcDate(newDate(d.y, 0, 1)).getUTCDay() : localDate(newDate(d.y, 0, 1)).getDay();
        d.m = 0;
        d.d = "W" in d ? (d.w + 6) % 7 + d.W * 7 - (day + 5) % 7 : d.w + d.U * 7 - (day + 6) % 7;
      }
      if ("Z" in d) {
        d.H += d.Z / 100 | 0;
        d.M += d.Z % 100;
        return utcDate(d);
      }
      return localDate(d);
    };
  }
  function parseSpecifier(d, specifier, string, j) {
    var i = 0, n = specifier.length, m3 = string.length, c3, parse;
    while (i < n) {
      if (j >= m3)
        return -1;
      c3 = specifier.charCodeAt(i++);
      if (c3 === 37) {
        c3 = specifier.charAt(i++);
        parse = parses[c3 in pads ? specifier.charAt(i++) : c3];
        if (!parse || (j = parse(d, string, j)) < 0)
          return -1;
      } else if (c3 != string.charCodeAt(j++)) {
        return -1;
      }
    }
    return j;
  }
  function parsePeriod(d, string, i) {
    var n = periodRe.exec(string.slice(i));
    return n ? (d.p = periodLookup.get(n[0].toLowerCase()), i + n[0].length) : -1;
  }
  function parseShortWeekday(d, string, i) {
    var n = shortWeekdayRe.exec(string.slice(i));
    return n ? (d.w = shortWeekdayLookup.get(n[0].toLowerCase()), i + n[0].length) : -1;
  }
  function parseWeekday(d, string, i) {
    var n = weekdayRe.exec(string.slice(i));
    return n ? (d.w = weekdayLookup.get(n[0].toLowerCase()), i + n[0].length) : -1;
  }
  function parseShortMonth(d, string, i) {
    var n = shortMonthRe.exec(string.slice(i));
    return n ? (d.m = shortMonthLookup.get(n[0].toLowerCase()), i + n[0].length) : -1;
  }
  function parseMonth(d, string, i) {
    var n = monthRe.exec(string.slice(i));
    return n ? (d.m = monthLookup.get(n[0].toLowerCase()), i + n[0].length) : -1;
  }
  function parseLocaleDateTime(d, string, i) {
    return parseSpecifier(d, locale_dateTime, string, i);
  }
  function parseLocaleDate(d, string, i) {
    return parseSpecifier(d, locale_date, string, i);
  }
  function parseLocaleTime(d, string, i) {
    return parseSpecifier(d, locale_time, string, i);
  }
  function formatShortWeekday(d) {
    return locale_shortWeekdays[d.getDay()];
  }
  function formatWeekday(d) {
    return locale_weekdays[d.getDay()];
  }
  function formatShortMonth(d) {
    return locale_shortMonths[d.getMonth()];
  }
  function formatMonth(d) {
    return locale_months[d.getMonth()];
  }
  function formatPeriod(d) {
    return locale_periods[+(d.getHours() >= 12)];
  }
  function formatQuarter(d) {
    return 1 + ~~(d.getMonth() / 3);
  }
  function formatUTCShortWeekday(d) {
    return locale_shortWeekdays[d.getUTCDay()];
  }
  function formatUTCWeekday(d) {
    return locale_weekdays[d.getUTCDay()];
  }
  function formatUTCShortMonth(d) {
    return locale_shortMonths[d.getUTCMonth()];
  }
  function formatUTCMonth(d) {
    return locale_months[d.getUTCMonth()];
  }
  function formatUTCPeriod(d) {
    return locale_periods[+(d.getUTCHours() >= 12)];
  }
  function formatUTCQuarter(d) {
    return 1 + ~~(d.getUTCMonth() / 3);
  }
  return {
    format: function(specifier) {
      var f = newFormat(specifier += "", formats);
      f.toString = function() {
        return specifier;
      };
      return f;
    },
    parse: function(specifier) {
      var p = newParse(specifier += "", false);
      p.toString = function() {
        return specifier;
      };
      return p;
    },
    utcFormat: function(specifier) {
      var f = newFormat(specifier += "", utcFormats);
      f.toString = function() {
        return specifier;
      };
      return f;
    },
    utcParse: function(specifier) {
      var p = newParse(specifier += "", true);
      p.toString = function() {
        return specifier;
      };
      return p;
    }
  };
}
var pads = { "-": "", _: " ", "0": "0" };
var numberRe = /^\s*\d+/;
var percentRe = /^%/;
var requoteRe = /[\\^$*+?|[\]().{}]/g;
function pad2(value, fill, width) {
  var sign2 = value < 0 ? "-" : "", string = (sign2 ? -value : value) + "", length3 = string.length;
  return sign2 + (length3 < width ? new Array(width - length3 + 1).join(fill) + string : string);
}
function requote(s) {
  return s.replace(requoteRe, "\\$&");
}
function formatRe(names) {
  return new RegExp("^(?:" + names.map(requote).join("|") + ")", "i");
}
function formatLookup(names) {
  return new Map(names.map((name, i) => [name.toLowerCase(), i]));
}
function parseWeekdayNumberSunday(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 1));
  return n ? (d.w = +n[0], i + n[0].length) : -1;
}
function parseWeekdayNumberMonday(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 1));
  return n ? (d.u = +n[0], i + n[0].length) : -1;
}
function parseWeekNumberSunday(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 2));
  return n ? (d.U = +n[0], i + n[0].length) : -1;
}
function parseWeekNumberISO(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 2));
  return n ? (d.V = +n[0], i + n[0].length) : -1;
}
function parseWeekNumberMonday(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 2));
  return n ? (d.W = +n[0], i + n[0].length) : -1;
}
function parseFullYear(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 4));
  return n ? (d.y = +n[0], i + n[0].length) : -1;
}
function parseYear(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 2));
  return n ? (d.y = +n[0] + (+n[0] > 68 ? 1900 : 2000), i + n[0].length) : -1;
}
function parseZone(d, string, i) {
  var n = /^(Z)|([+-]\d\d)(?::?(\d\d))?/.exec(string.slice(i, i + 6));
  return n ? (d.Z = n[1] ? 0 : -(n[2] + (n[3] || "00")), i + n[0].length) : -1;
}
function parseQuarter(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 1));
  return n ? (d.q = n[0] * 3 - 3, i + n[0].length) : -1;
}
function parseMonthNumber(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 2));
  return n ? (d.m = n[0] - 1, i + n[0].length) : -1;
}
function parseDayOfMonth(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 2));
  return n ? (d.d = +n[0], i + n[0].length) : -1;
}
function parseDayOfYear(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 3));
  return n ? (d.m = 0, d.d = +n[0], i + n[0].length) : -1;
}
function parseHour24(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 2));
  return n ? (d.H = +n[0], i + n[0].length) : -1;
}
function parseMinutes(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 2));
  return n ? (d.M = +n[0], i + n[0].length) : -1;
}
function parseSeconds(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 2));
  return n ? (d.S = +n[0], i + n[0].length) : -1;
}
function parseMilliseconds(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 3));
  return n ? (d.L = +n[0], i + n[0].length) : -1;
}
function parseMicroseconds(d, string, i) {
  var n = numberRe.exec(string.slice(i, i + 6));
  return n ? (d.L = Math.floor(n[0] / 1000), i + n[0].length) : -1;
}
function parseLiteralPercent(d, string, i) {
  var n = percentRe.exec(string.slice(i, i + 1));
  return n ? i + n[0].length : -1;
}
function parseUnixTimestamp(d, string, i) {
  var n = numberRe.exec(string.slice(i));
  return n ? (d.Q = +n[0], i + n[0].length) : -1;
}
function parseUnixTimestampSeconds(d, string, i) {
  var n = numberRe.exec(string.slice(i));
  return n ? (d.s = +n[0], i + n[0].length) : -1;
}
function formatDayOfMonth(d, p) {
  return pad2(d.getDate(), p, 2);
}
function formatHour24(d, p) {
  return pad2(d.getHours(), p, 2);
}
function formatHour12(d, p) {
  return pad2(d.getHours() % 12 || 12, p, 2);
}
function formatDayOfYear(d, p) {
  return pad2(1 + timeDay.count(timeYear(d), d), p, 3);
}
function formatMilliseconds(d, p) {
  return pad2(d.getMilliseconds(), p, 3);
}
function formatMicroseconds(d, p) {
  return formatMilliseconds(d, p) + "000";
}
function formatMonthNumber(d, p) {
  return pad2(d.getMonth() + 1, p, 2);
}
function formatMinutes(d, p) {
  return pad2(d.getMinutes(), p, 2);
}
function formatSeconds(d, p) {
  return pad2(d.getSeconds(), p, 2);
}
function formatWeekdayNumberMonday(d) {
  var day = d.getDay();
  return day === 0 ? 7 : day;
}
function formatWeekNumberSunday(d, p) {
  return pad2(timeSunday.count(timeYear(d) - 1, d), p, 2);
}
function dISO(d) {
  var day = d.getDay();
  return day >= 4 || day === 0 ? timeThursday(d) : timeThursday.ceil(d);
}
function formatWeekNumberISO(d, p) {
  d = dISO(d);
  return pad2(timeThursday.count(timeYear(d), d) + (timeYear(d).getDay() === 4), p, 2);
}
function formatWeekdayNumberSunday(d) {
  return d.getDay();
}
function formatWeekNumberMonday(d, p) {
  return pad2(timeMonday.count(timeYear(d) - 1, d), p, 2);
}
function formatYear2(d, p) {
  return pad2(d.getFullYear() % 100, p, 2);
}
function formatYearISO(d, p) {
  d = dISO(d);
  return pad2(d.getFullYear() % 100, p, 2);
}
function formatFullYear(d, p) {
  return pad2(d.getFullYear() % 1e4, p, 4);
}
function formatFullYearISO(d, p) {
  var day = d.getDay();
  d = day >= 4 || day === 0 ? timeThursday(d) : timeThursday.ceil(d);
  return pad2(d.getFullYear() % 1e4, p, 4);
}
function formatZone(d) {
  var z = d.getTimezoneOffset();
  return (z > 0 ? "-" : (z *= -1, "+")) + pad2(z / 60 | 0, "0", 2) + pad2(z % 60, "0", 2);
}
function formatUTCDayOfMonth(d, p) {
  return pad2(d.getUTCDate(), p, 2);
}
function formatUTCHour24(d, p) {
  return pad2(d.getUTCHours(), p, 2);
}
function formatUTCHour12(d, p) {
  return pad2(d.getUTCHours() % 12 || 12, p, 2);
}
function formatUTCDayOfYear(d, p) {
  return pad2(1 + utcDay.count(utcYear(d), d), p, 3);
}
function formatUTCMilliseconds(d, p) {
  return pad2(d.getUTCMilliseconds(), p, 3);
}
function formatUTCMicroseconds(d, p) {
  return formatUTCMilliseconds(d, p) + "000";
}
function formatUTCMonthNumber(d, p) {
  return pad2(d.getUTCMonth() + 1, p, 2);
}
function formatUTCMinutes(d, p) {
  return pad2(d.getUTCMinutes(), p, 2);
}
function formatUTCSeconds(d, p) {
  return pad2(d.getUTCSeconds(), p, 2);
}
function formatUTCWeekdayNumberMonday(d) {
  var dow = d.getUTCDay();
  return dow === 0 ? 7 : dow;
}
function formatUTCWeekNumberSunday(d, p) {
  return pad2(utcSunday.count(utcYear(d) - 1, d), p, 2);
}
function UTCdISO(d) {
  var day = d.getUTCDay();
  return day >= 4 || day === 0 ? utcThursday(d) : utcThursday.ceil(d);
}
function formatUTCWeekNumberISO(d, p) {
  d = UTCdISO(d);
  return pad2(utcThursday.count(utcYear(d), d) + (utcYear(d).getUTCDay() === 4), p, 2);
}
function formatUTCWeekdayNumberSunday(d) {
  return d.getUTCDay();
}
function formatUTCWeekNumberMonday(d, p) {
  return pad2(utcMonday.count(utcYear(d) - 1, d), p, 2);
}
function formatUTCYear(d, p) {
  return pad2(d.getUTCFullYear() % 100, p, 2);
}
function formatUTCYearISO(d, p) {
  d = UTCdISO(d);
  return pad2(d.getUTCFullYear() % 100, p, 2);
}
function formatUTCFullYear(d, p) {
  return pad2(d.getUTCFullYear() % 1e4, p, 4);
}
function formatUTCFullYearISO(d, p) {
  var day = d.getUTCDay();
  d = day >= 4 || day === 0 ? utcThursday(d) : utcThursday.ceil(d);
  return pad2(d.getUTCFullYear() % 1e4, p, 4);
}
function formatUTCZone() {
  return "+0000";
}
function formatLiteralPercent() {
  return "%";
}
function formatUnixTimestamp(d) {
  return +d;
}
function formatUnixTimestampSeconds(d) {
  return Math.floor(+d / 1000);
}

// node_modules/d3-time-format/src/defaultLocale.js
var locale2;
var timeFormat;
var timeParse;
var utcFormat;
var utcParse;
defaultLocale2({
  dateTime: "%x, %X",
  date: "%-m/%-d/%Y",
  time: "%-I:%M:%S %p",
  periods: ["AM", "PM"],
  days: ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"],
  shortDays: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
  months: ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"],
  shortMonths: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
});
function defaultLocale2(definition) {
  locale2 = formatLocale(definition);
  timeFormat = locale2.format;
  timeParse = locale2.parse;
  utcFormat = locale2.utcFormat;
  utcParse = locale2.utcParse;
  return locale2;
}
// node_modules/d3-time-format/src/isoFormat.js
var isoSpecifier = "%Y-%m-%dT%H:%M:%S.%LZ";
function formatIsoNative(date) {
  return date.toISOString();
}
var formatIso = Date.prototype.toISOString ? formatIsoNative : utcFormat(isoSpecifier);
var isoFormat_default = formatIso;
// node_modules/d3-time-format/src/isoParse.js
function parseIsoNative(string) {
  var date = new Date(string);
  return isNaN(date) ? null : date;
}
var parseIso = +new Date("2000-01-01T00:00:00.000Z") ? parseIsoNative : utcParse(isoSpecifier);
var isoParse_default = parseIso;
// node_modules/d3-scale/src/time.js
function date(t) {
  return new Date(t);
}
function number4(t) {
  return t instanceof Date ? +t : +new Date(+t);
}
function calendar(ticks2, tickInterval, year, month, week, day, hour, minute, second2, format2) {
  var scale2 = continuous(), invert = scale2.invert, domain = scale2.domain;
  var formatMillisecond = format2(".%L"), formatSecond = format2(":%S"), formatMinute = format2("%I:%M"), formatHour = format2("%I %p"), formatDay = format2("%a %d"), formatWeek = format2("%b %d"), formatMonth = format2("%B"), formatYear3 = format2("%Y");
  function tickFormat2(date2) {
    return (second2(date2) < date2 ? formatMillisecond : minute(date2) < date2 ? formatSecond : hour(date2) < date2 ? formatMinute : day(date2) < date2 ? formatHour : month(date2) < date2 ? week(date2) < date2 ? formatDay : formatWeek : year(date2) < date2 ? formatMonth : formatYear3)(date2);
  }
  scale2.invert = function(y3) {
    return new Date(invert(y3));
  };
  scale2.domain = function(_) {
    return arguments.length ? domain(Array.from(_, number4)) : domain().map(date);
  };
  scale2.ticks = function(interval2) {
    var d = domain();
    return ticks2(d[0], d[d.length - 1], interval2 == null ? 10 : interval2);
  };
  scale2.tickFormat = function(count3, specifier) {
    return specifier == null ? tickFormat2 : format2(specifier);
  };
  scale2.nice = function(interval2) {
    var d = domain();
    if (!interval2 || typeof interval2.range !== "function")
      interval2 = tickInterval(d[0], d[d.length - 1], interval2 == null ? 10 : interval2);
    return interval2 ? domain(nice2(d, interval2)) : scale2;
  };
  scale2.copy = function() {
    return copy(scale2, calendar(ticks2, tickInterval, year, month, week, day, hour, minute, second2, format2));
  };
  return scale2;
}
function time() {
  return initRange.apply(calendar(timeTicks, timeTickInterval, timeYear, timeMonth, timeSunday, timeDay, timeHour, timeMinute, second, timeFormat).domain([new Date(2000, 0, 1), new Date(2000, 0, 2)]), arguments);
}
// node_modules/d3-scale/src/utcTime.js
function utcTime() {
  return initRange.apply(calendar(utcTicks, utcTickInterval, utcYear, utcMonth, utcSunday, utcDay, utcHour, utcMinute, second, utcFormat).domain([Date.UTC(2000, 0, 1), Date.UTC(2000, 0, 2)]), arguments);
}
// node_modules/d3-scale/src/sequential.js
function transformer3() {
  var x06 = 0, x12 = 1, t03, t13, k10, transform, interpolator = identity3, clamp = false, unknown;
  function scale2(x3) {
    return x3 == null || isNaN(x3 = +x3) ? unknown : interpolator(k10 === 0 ? 0.5 : (x3 = (transform(x3) - t03) * k10, clamp ? Math.max(0, Math.min(1, x3)) : x3));
  }
  scale2.domain = function(_) {
    return arguments.length ? ([x06, x12] = _, t03 = transform(x06 = +x06), t13 = transform(x12 = +x12), k10 = t03 === t13 ? 0 : 1 / (t13 - t03), scale2) : [x06, x12];
  };
  scale2.clamp = function(_) {
    return arguments.length ? (clamp = !!_, scale2) : clamp;
  };
  scale2.interpolator = function(_) {
    return arguments.length ? (interpolator = _, scale2) : interpolator;
  };
  function range4(interpolate) {
    return function(_) {
      var r0, r1;
      return arguments.length ? ([r0, r1] = _, interpolator = interpolate(r0, r1), scale2) : [interpolator(0), interpolator(1)];
    };
  }
  scale2.range = range4(value_default);
  scale2.rangeRound = range4(round_default);
  scale2.unknown = function(_) {
    return arguments.length ? (unknown = _, scale2) : unknown;
  };
  return function(t) {
    transform = t, t03 = t(x06), t13 = t(x12), k10 = t03 === t13 ? 0 : 1 / (t13 - t03);
    return scale2;
  };
}
function copy2(source, target) {
  return target.domain(source.domain()).interpolator(source.interpolator()).clamp(source.clamp()).unknown(source.unknown());
}
function sequential() {
  var scale2 = linearish(transformer3()(identity3));
  scale2.copy = function() {
    return copy2(scale2, sequential());
  };
  return initInterpolator.apply(scale2, arguments);
}
function sequentialLog() {
  var scale2 = loggish(transformer3()).domain([1, 10]);
  scale2.copy = function() {
    return copy2(scale2, sequentialLog()).base(scale2.base());
  };
  return initInterpolator.apply(scale2, arguments);
}
function sequentialSymlog() {
  var scale2 = symlogish(transformer3());
  scale2.copy = function() {
    return copy2(scale2, sequentialSymlog()).constant(scale2.constant());
  };
  return initInterpolator.apply(scale2, arguments);
}
function sequentialPow() {
  var scale2 = powish(transformer3());
  scale2.copy = function() {
    return copy2(scale2, sequentialPow()).exponent(scale2.exponent());
  };
  return initInterpolator.apply(scale2, arguments);
}
function sequentialSqrt() {
  return sequentialPow.apply(null, arguments).exponent(0.5);
}
// node_modules/d3-scale/src/sequentialQuantile.js
function sequentialQuantile() {
  var domain = [], interpolator = identity3;
  function scale2(x3) {
    if (x3 != null && !isNaN(x3 = +x3))
      return interpolator((bisect_default(domain, x3, 1) - 1) / (domain.length - 1));
  }
  scale2.domain = function(_) {
    if (!arguments.length)
      return domain.slice();
    domain = [];
    for (let d of _)
      if (d != null && !isNaN(d = +d))
        domain.push(d);
    domain.sort(ascending);
    return scale2;
  };
  scale2.interpolator = function(_) {
    return arguments.length ? (interpolator = _, scale2) : interpolator;
  };
  scale2.range = function() {
    return domain.map((d, i) => interpolator(i / (domain.length - 1)));
  };
  scale2.quantiles = function(n) {
    return Array.from({ length: n + 1 }, (_, i) => quantile(domain, i / n));
  };
  scale2.copy = function() {
    return sequentialQuantile(interpolator).domain(domain);
  };
  return initInterpolator.apply(scale2, arguments);
}
// node_modules/d3-scale/src/diverging.js
function transformer4() {
  var x06 = 0, x12 = 0.5, x22 = 1, s = 1, t03, t13, t22, k10, k21, interpolator = identity3, transform, clamp = false, unknown;
  function scale2(x3) {
    return isNaN(x3 = +x3) ? unknown : (x3 = 0.5 + ((x3 = +transform(x3)) - t13) * (s * x3 < s * t13 ? k10 : k21), interpolator(clamp ? Math.max(0, Math.min(1, x3)) : x3));
  }
  scale2.domain = function(_) {
    return arguments.length ? ([x06, x12, x22] = _, t03 = transform(x06 = +x06), t13 = transform(x12 = +x12), t22 = transform(x22 = +x22), k10 = t03 === t13 ? 0 : 0.5 / (t13 - t03), k21 = t13 === t22 ? 0 : 0.5 / (t22 - t13), s = t13 < t03 ? -1 : 1, scale2) : [x06, x12, x22];
  };
  scale2.clamp = function(_) {
    return arguments.length ? (clamp = !!_, scale2) : clamp;
  };
  scale2.interpolator = function(_) {
    return arguments.length ? (interpolator = _, scale2) : interpolator;
  };
  function range4(interpolate) {
    return function(_) {
      var r0, r1, r2;
      return arguments.length ? ([r0, r1, r2] = _, interpolator = piecewise(interpolate, [r0, r1, r2]), scale2) : [interpolator(0), interpolator(0.5), interpolator(1)];
    };
  }
  scale2.range = range4(value_default);
  scale2.rangeRound = range4(round_default);
  scale2.unknown = function(_) {
    return arguments.length ? (unknown = _, scale2) : unknown;
  };
  return function(t) {
    transform = t, t03 = t(x06), t13 = t(x12), t22 = t(x22), k10 = t03 === t13 ? 0 : 0.5 / (t13 - t03), k21 = t13 === t22 ? 0 : 0.5 / (t22 - t13), s = t13 < t03 ? -1 : 1;
    return scale2;
  };
}
function diverging() {
  var scale2 = linearish(transformer4()(identity3));
  scale2.copy = function() {
    return copy2(scale2, diverging());
  };
  return initInterpolator.apply(scale2, arguments);
}
function divergingLog() {
  var scale2 = loggish(transformer4()).domain([0.1, 1, 10]);
  scale2.copy = function() {
    return copy2(scale2, divergingLog()).base(scale2.base());
  };
  return initInterpolator.apply(scale2, arguments);
}
function divergingSymlog() {
  var scale2 = symlogish(transformer4());
  scale2.copy = function() {
    return copy2(scale2, divergingSymlog()).constant(scale2.constant());
  };
  return initInterpolator.apply(scale2, arguments);
}
function divergingPow() {
  var scale2 = powish(transformer4());
  scale2.copy = function() {
    return copy2(scale2, divergingPow()).exponent(scale2.exponent());
  };
  return initInterpolator.apply(scale2, arguments);
}
function divergingSqrt() {
  return divergingPow.apply(null, arguments).exponent(0.5);
}
// node_modules/d3-scale-chromatic/src/colors.js
function colors_default(specifier) {
  var n = specifier.length / 6 | 0, colors = new Array(n), i = 0;
  while (i < n)
    colors[i] = "#" + specifier.slice(i * 6, ++i * 6);
  return colors;
}

// node_modules/d3-scale-chromatic/src/categorical/category10.js
var category10_default = colors_default("1f77b4ff7f0e2ca02cd627289467bd8c564be377c27f7f7fbcbd2217becf");
// node_modules/d3-scale-chromatic/src/categorical/Accent.js
var Accent_default = colors_default("7fc97fbeaed4fdc086ffff99386cb0f0027fbf5b17666666");
// node_modules/d3-scale-chromatic/src/categorical/Dark2.js
var Dark2_default = colors_default("1b9e77d95f027570b3e7298a66a61ee6ab02a6761d666666");
// node_modules/d3-scale-chromatic/src/categorical/observable10.js
var observable10_default = colors_default("4269d0efb118ff725c6cc5b03ca951ff8ab7a463f297bbf59c6b4e9498a0");
// node_modules/d3-scale-chromatic/src/categorical/Paired.js
var Paired_default = colors_default("a6cee31f78b4b2df8a33a02cfb9a99e31a1cfdbf6fff7f00cab2d66a3d9affff99b15928");
// node_modules/d3-scale-chromatic/src/categorical/Pastel1.js
var Pastel1_default = colors_default("fbb4aeb3cde3ccebc5decbe4fed9a6ffffcce5d8bdfddaecf2f2f2");
// node_modules/d3-scale-chromatic/src/categorical/Pastel2.js
var Pastel2_default = colors_default("b3e2cdfdcdaccbd5e8f4cae4e6f5c9fff2aef1e2cccccccc");
// node_modules/d3-scale-chromatic/src/categorical/Set1.js
var Set1_default = colors_default("e41a1c377eb84daf4a984ea3ff7f00ffff33a65628f781bf999999");
// node_modules/d3-scale-chromatic/src/categorical/Set2.js
var Set2_default = colors_default("66c2a5fc8d628da0cbe78ac3a6d854ffd92fe5c494b3b3b3");
// node_modules/d3-scale-chromatic/src/categorical/Set3.js
var Set3_default = colors_default("8dd3c7ffffb3bebadafb807280b1d3fdb462b3de69fccde5d9d9d9bc80bdccebc5ffed6f");
// node_modules/d3-scale-chromatic/src/categorical/Tableau10.js
var Tableau10_default = colors_default("4e79a7f28e2ce1575976b7b259a14fedc949af7aa1ff9da79c755fbab0ab");
// node_modules/d3-scale-chromatic/src/ramp.js
var ramp_default = (scheme) => rgbBasis(scheme[scheme.length - 1]);

// node_modules/d3-scale-chromatic/src/diverging/BrBG.js
var scheme = new Array(3).concat("d8b365f5f5f55ab4ac", "a6611adfc27d80cdc1018571", "a6611adfc27df5f5f580cdc1018571", "8c510ad8b365f6e8c3c7eae55ab4ac01665e", "8c510ad8b365f6e8c3f5f5f5c7eae55ab4ac01665e", "8c510abf812ddfc27df6e8c3c7eae580cdc135978f01665e", "8c510abf812ddfc27df6e8c3f5f5f5c7eae580cdc135978f01665e", "5430058c510abf812ddfc27df6e8c3c7eae580cdc135978f01665e003c30", "5430058c510abf812ddfc27df6e8c3f5f5f5c7eae580cdc135978f01665e003c30").map(colors_default);
var BrBG_default = ramp_default(scheme);
// node_modules/d3-scale-chromatic/src/diverging/PRGn.js
var scheme2 = new Array(3).concat("af8dc3f7f7f77fbf7b", "7b3294c2a5cfa6dba0008837", "7b3294c2a5cff7f7f7a6dba0008837", "762a83af8dc3e7d4e8d9f0d37fbf7b1b7837", "762a83af8dc3e7d4e8f7f7f7d9f0d37fbf7b1b7837", "762a839970abc2a5cfe7d4e8d9f0d3a6dba05aae611b7837", "762a839970abc2a5cfe7d4e8f7f7f7d9f0d3a6dba05aae611b7837", "40004b762a839970abc2a5cfe7d4e8d9f0d3a6dba05aae611b783700441b", "40004b762a839970abc2a5cfe7d4e8f7f7f7d9f0d3a6dba05aae611b783700441b").map(colors_default);
var PRGn_default = ramp_default(scheme2);
// node_modules/d3-scale-chromatic/src/diverging/PiYG.js
var scheme3 = new Array(3).concat("e9a3c9f7f7f7a1d76a", "d01c8bf1b6dab8e1864dac26", "d01c8bf1b6daf7f7f7b8e1864dac26", "c51b7de9a3c9fde0efe6f5d0a1d76a4d9221", "c51b7de9a3c9fde0eff7f7f7e6f5d0a1d76a4d9221", "c51b7dde77aef1b6dafde0efe6f5d0b8e1867fbc414d9221", "c51b7dde77aef1b6dafde0eff7f7f7e6f5d0b8e1867fbc414d9221", "8e0152c51b7dde77aef1b6dafde0efe6f5d0b8e1867fbc414d9221276419", "8e0152c51b7dde77aef1b6dafde0eff7f7f7e6f5d0b8e1867fbc414d9221276419").map(colors_default);
var PiYG_default = ramp_default(scheme3);
// node_modules/d3-scale-chromatic/src/diverging/PuOr.js
var scheme4 = new Array(3).concat("998ec3f7f7f7f1a340", "5e3c99b2abd2fdb863e66101", "5e3c99b2abd2f7f7f7fdb863e66101", "542788998ec3d8daebfee0b6f1a340b35806", "542788998ec3d8daebf7f7f7fee0b6f1a340b35806", "5427888073acb2abd2d8daebfee0b6fdb863e08214b35806", "5427888073acb2abd2d8daebf7f7f7fee0b6fdb863e08214b35806", "2d004b5427888073acb2abd2d8daebfee0b6fdb863e08214b358067f3b08", "2d004b5427888073acb2abd2d8daebf7f7f7fee0b6fdb863e08214b358067f3b08").map(colors_default);
var PuOr_default = ramp_default(scheme4);
// node_modules/d3-scale-chromatic/src/diverging/RdBu.js
var scheme5 = new Array(3).concat("ef8a62f7f7f767a9cf", "ca0020f4a58292c5de0571b0", "ca0020f4a582f7f7f792c5de0571b0", "b2182bef8a62fddbc7d1e5f067a9cf2166ac", "b2182bef8a62fddbc7f7f7f7d1e5f067a9cf2166ac", "b2182bd6604df4a582fddbc7d1e5f092c5de4393c32166ac", "b2182bd6604df4a582fddbc7f7f7f7d1e5f092c5de4393c32166ac", "67001fb2182bd6604df4a582fddbc7d1e5f092c5de4393c32166ac053061", "67001fb2182bd6604df4a582fddbc7f7f7f7d1e5f092c5de4393c32166ac053061").map(colors_default);
var RdBu_default = ramp_default(scheme5);
// node_modules/d3-scale-chromatic/src/diverging/RdGy.js
var scheme6 = new Array(3).concat("ef8a62ffffff999999", "ca0020f4a582bababa404040", "ca0020f4a582ffffffbababa404040", "b2182bef8a62fddbc7e0e0e09999994d4d4d", "b2182bef8a62fddbc7ffffffe0e0e09999994d4d4d", "b2182bd6604df4a582fddbc7e0e0e0bababa8787874d4d4d", "b2182bd6604df4a582fddbc7ffffffe0e0e0bababa8787874d4d4d", "67001fb2182bd6604df4a582fddbc7e0e0e0bababa8787874d4d4d1a1a1a", "67001fb2182bd6604df4a582fddbc7ffffffe0e0e0bababa8787874d4d4d1a1a1a").map(colors_default);
var RdGy_default = ramp_default(scheme6);
// node_modules/d3-scale-chromatic/src/diverging/RdYlBu.js
var scheme7 = new Array(3).concat("fc8d59ffffbf91bfdb", "d7191cfdae61abd9e92c7bb6", "d7191cfdae61ffffbfabd9e92c7bb6", "d73027fc8d59fee090e0f3f891bfdb4575b4", "d73027fc8d59fee090ffffbfe0f3f891bfdb4575b4", "d73027f46d43fdae61fee090e0f3f8abd9e974add14575b4", "d73027f46d43fdae61fee090ffffbfe0f3f8abd9e974add14575b4", "a50026d73027f46d43fdae61fee090e0f3f8abd9e974add14575b4313695", "a50026d73027f46d43fdae61fee090ffffbfe0f3f8abd9e974add14575b4313695").map(colors_default);
var RdYlBu_default = ramp_default(scheme7);
// node_modules/d3-scale-chromatic/src/diverging/RdYlGn.js
var scheme8 = new Array(3).concat("fc8d59ffffbf91cf60", "d7191cfdae61a6d96a1a9641", "d7191cfdae61ffffbfa6d96a1a9641", "d73027fc8d59fee08bd9ef8b91cf601a9850", "d73027fc8d59fee08bffffbfd9ef8b91cf601a9850", "d73027f46d43fdae61fee08bd9ef8ba6d96a66bd631a9850", "d73027f46d43fdae61fee08bffffbfd9ef8ba6d96a66bd631a9850", "a50026d73027f46d43fdae61fee08bd9ef8ba6d96a66bd631a9850006837", "a50026d73027f46d43fdae61fee08bffffbfd9ef8ba6d96a66bd631a9850006837").map(colors_default);
var RdYlGn_default = ramp_default(scheme8);
// node_modules/d3-scale-chromatic/src/diverging/Spectral.js
var scheme9 = new Array(3).concat("fc8d59ffffbf99d594", "d7191cfdae61abdda42b83ba", "d7191cfdae61ffffbfabdda42b83ba", "d53e4ffc8d59fee08be6f59899d5943288bd", "d53e4ffc8d59fee08bffffbfe6f59899d5943288bd", "d53e4ff46d43fdae61fee08be6f598abdda466c2a53288bd", "d53e4ff46d43fdae61fee08bffffbfe6f598abdda466c2a53288bd", "9e0142d53e4ff46d43fdae61fee08be6f598abdda466c2a53288bd5e4fa2", "9e0142d53e4ff46d43fdae61fee08bffffbfe6f598abdda466c2a53288bd5e4fa2").map(colors_default);
var Spectral_default = ramp_default(scheme9);
// node_modules/d3-scale-chromatic/src/sequential-multi/BuGn.js
var scheme10 = new Array(3).concat("e5f5f999d8c92ca25f", "edf8fbb2e2e266c2a4238b45", "edf8fbb2e2e266c2a42ca25f006d2c", "edf8fbccece699d8c966c2a42ca25f006d2c", "edf8fbccece699d8c966c2a441ae76238b45005824", "f7fcfde5f5f9ccece699d8c966c2a441ae76238b45005824", "f7fcfde5f5f9ccece699d8c966c2a441ae76238b45006d2c00441b").map(colors_default);
var BuGn_default = ramp_default(scheme10);
// node_modules/d3-scale-chromatic/src/sequential-multi/BuPu.js
var scheme11 = new Array(3).concat("e0ecf49ebcda8856a7", "edf8fbb3cde38c96c688419d", "edf8fbb3cde38c96c68856a7810f7c", "edf8fbbfd3e69ebcda8c96c68856a7810f7c", "edf8fbbfd3e69ebcda8c96c68c6bb188419d6e016b", "f7fcfde0ecf4bfd3e69ebcda8c96c68c6bb188419d6e016b", "f7fcfde0ecf4bfd3e69ebcda8c96c68c6bb188419d810f7c4d004b").map(colors_default);
var BuPu_default = ramp_default(scheme11);
// node_modules/d3-scale-chromatic/src/sequential-multi/GnBu.js
var scheme12 = new Array(3).concat("e0f3dba8ddb543a2ca", "f0f9e8bae4bc7bccc42b8cbe", "f0f9e8bae4bc7bccc443a2ca0868ac", "f0f9e8ccebc5a8ddb57bccc443a2ca0868ac", "f0f9e8ccebc5a8ddb57bccc44eb3d32b8cbe08589e", "f7fcf0e0f3dbccebc5a8ddb57bccc44eb3d32b8cbe08589e", "f7fcf0e0f3dbccebc5a8ddb57bccc44eb3d32b8cbe0868ac084081").map(colors_default);
var GnBu_default = ramp_default(scheme12);
// node_modules/d3-scale-chromatic/src/sequential-multi/OrRd.js
var scheme13 = new Array(3).concat("fee8c8fdbb84e34a33", "fef0d9fdcc8afc8d59d7301f", "fef0d9fdcc8afc8d59e34a33b30000", "fef0d9fdd49efdbb84fc8d59e34a33b30000", "fef0d9fdd49efdbb84fc8d59ef6548d7301f990000", "fff7ecfee8c8fdd49efdbb84fc8d59ef6548d7301f990000", "fff7ecfee8c8fdd49efdbb84fc8d59ef6548d7301fb300007f0000").map(colors_default);
var OrRd_default = ramp_default(scheme13);
// node_modules/d3-scale-chromatic/src/sequential-multi/PuBuGn.js
var scheme14 = new Array(3).concat("ece2f0a6bddb1c9099", "f6eff7bdc9e167a9cf02818a", "f6eff7bdc9e167a9cf1c9099016c59", "f6eff7d0d1e6a6bddb67a9cf1c9099016c59", "f6eff7d0d1e6a6bddb67a9cf3690c002818a016450", "fff7fbece2f0d0d1e6a6bddb67a9cf3690c002818a016450", "fff7fbece2f0d0d1e6a6bddb67a9cf3690c002818a016c59014636").map(colors_default);
var PuBuGn_default = ramp_default(scheme14);
// node_modules/d3-scale-chromatic/src/sequential-multi/PuBu.js
var scheme15 = new Array(3).concat("ece7f2a6bddb2b8cbe", "f1eef6bdc9e174a9cf0570b0", "f1eef6bdc9e174a9cf2b8cbe045a8d", "f1eef6d0d1e6a6bddb74a9cf2b8cbe045a8d", "f1eef6d0d1e6a6bddb74a9cf3690c00570b0034e7b", "fff7fbece7f2d0d1e6a6bddb74a9cf3690c00570b0034e7b", "fff7fbece7f2d0d1e6a6bddb74a9cf3690c00570b0045a8d023858").map(colors_default);
var PuBu_default = ramp_default(scheme15);
// node_modules/d3-scale-chromatic/src/sequential-multi/PuRd.js
var scheme16 = new Array(3).concat("e7e1efc994c7dd1c77", "f1eef6d7b5d8df65b0ce1256", "f1eef6d7b5d8df65b0dd1c77980043", "f1eef6d4b9dac994c7df65b0dd1c77980043", "f1eef6d4b9dac994c7df65b0e7298ace125691003f", "f7f4f9e7e1efd4b9dac994c7df65b0e7298ace125691003f", "f7f4f9e7e1efd4b9dac994c7df65b0e7298ace125698004367001f").map(colors_default);
var PuRd_default = ramp_default(scheme16);
// node_modules/d3-scale-chromatic/src/sequential-multi/RdPu.js
var scheme17 = new Array(3).concat("fde0ddfa9fb5c51b8a", "feebe2fbb4b9f768a1ae017e", "feebe2fbb4b9f768a1c51b8a7a0177", "feebe2fcc5c0fa9fb5f768a1c51b8a7a0177", "feebe2fcc5c0fa9fb5f768a1dd3497ae017e7a0177", "fff7f3fde0ddfcc5c0fa9fb5f768a1dd3497ae017e7a0177", "fff7f3fde0ddfcc5c0fa9fb5f768a1dd3497ae017e7a017749006a").map(colors_default);
var RdPu_default = ramp_default(scheme17);
// node_modules/d3-scale-chromatic/src/sequential-multi/YlGnBu.js
var scheme18 = new Array(3).concat("edf8b17fcdbb2c7fb8", "ffffcca1dab441b6c4225ea8", "ffffcca1dab441b6c42c7fb8253494", "ffffccc7e9b47fcdbb41b6c42c7fb8253494", "ffffccc7e9b47fcdbb41b6c41d91c0225ea80c2c84", "ffffd9edf8b1c7e9b47fcdbb41b6c41d91c0225ea80c2c84", "ffffd9edf8b1c7e9b47fcdbb41b6c41d91c0225ea8253494081d58").map(colors_default);
var YlGnBu_default = ramp_default(scheme18);
// node_modules/d3-scale-chromatic/src/sequential-multi/YlGn.js
var scheme19 = new Array(3).concat("f7fcb9addd8e31a354", "ffffccc2e69978c679238443", "ffffccc2e69978c67931a354006837", "ffffccd9f0a3addd8e78c67931a354006837", "ffffccd9f0a3addd8e78c67941ab5d238443005a32", "ffffe5f7fcb9d9f0a3addd8e78c67941ab5d238443005a32", "ffffe5f7fcb9d9f0a3addd8e78c67941ab5d238443006837004529").map(colors_default);
var YlGn_default = ramp_default(scheme19);
// node_modules/d3-scale-chromatic/src/sequential-multi/YlOrBr.js
var scheme20 = new Array(3).concat("fff7bcfec44fd95f0e", "ffffd4fed98efe9929cc4c02", "ffffd4fed98efe9929d95f0e993404", "ffffd4fee391fec44ffe9929d95f0e993404", "ffffd4fee391fec44ffe9929ec7014cc4c028c2d04", "ffffe5fff7bcfee391fec44ffe9929ec7014cc4c028c2d04", "ffffe5fff7bcfee391fec44ffe9929ec7014cc4c02993404662506").map(colors_default);
var YlOrBr_default = ramp_default(scheme20);
// node_modules/d3-scale-chromatic/src/sequential-multi/YlOrRd.js
var scheme21 = new Array(3).concat("ffeda0feb24cf03b20", "ffffb2fecc5cfd8d3ce31a1c", "ffffb2fecc5cfd8d3cf03b20bd0026", "ffffb2fed976feb24cfd8d3cf03b20bd0026", "ffffb2fed976feb24cfd8d3cfc4e2ae31a1cb10026", "ffffccffeda0fed976feb24cfd8d3cfc4e2ae31a1cb10026", "ffffccffeda0fed976feb24cfd8d3cfc4e2ae31a1cbd0026800026").map(colors_default);
var YlOrRd_default = ramp_default(scheme21);
// node_modules/d3-scale-chromatic/src/sequential-single/Blues.js
var scheme22 = new Array(3).concat("deebf79ecae13182bd", "eff3ffbdd7e76baed62171b5", "eff3ffbdd7e76baed63182bd08519c", "eff3ffc6dbef9ecae16baed63182bd08519c", "eff3ffc6dbef9ecae16baed64292c62171b5084594", "f7fbffdeebf7c6dbef9ecae16baed64292c62171b5084594", "f7fbffdeebf7c6dbef9ecae16baed64292c62171b508519c08306b").map(colors_default);
var Blues_default = ramp_default(scheme22);
// node_modules/d3-scale-chromatic/src/sequential-single/Greens.js
var scheme23 = new Array(3).concat("e5f5e0a1d99b31a354", "edf8e9bae4b374c476238b45", "edf8e9bae4b374c47631a354006d2c", "edf8e9c7e9c0a1d99b74c47631a354006d2c", "edf8e9c7e9c0a1d99b74c47641ab5d238b45005a32", "f7fcf5e5f5e0c7e9c0a1d99b74c47641ab5d238b45005a32", "f7fcf5e5f5e0c7e9c0a1d99b74c47641ab5d238b45006d2c00441b").map(colors_default);
var Greens_default = ramp_default(scheme23);
// node_modules/d3-scale-chromatic/src/sequential-single/Greys.js
var scheme24 = new Array(3).concat("f0f0f0bdbdbd636363", "f7f7f7cccccc969696525252", "f7f7f7cccccc969696636363252525", "f7f7f7d9d9d9bdbdbd969696636363252525", "f7f7f7d9d9d9bdbdbd969696737373525252252525", "fffffff0f0f0d9d9d9bdbdbd969696737373525252252525", "fffffff0f0f0d9d9d9bdbdbd969696737373525252252525000000").map(colors_default);
var Greys_default = ramp_default(scheme24);
// node_modules/d3-scale-chromatic/src/sequential-single/Purples.js
var scheme25 = new Array(3).concat("efedf5bcbddc756bb1", "f2f0f7cbc9e29e9ac86a51a3", "f2f0f7cbc9e29e9ac8756bb154278f", "f2f0f7dadaebbcbddc9e9ac8756bb154278f", "f2f0f7dadaebbcbddc9e9ac8807dba6a51a34a1486", "fcfbfdefedf5dadaebbcbddc9e9ac8807dba6a51a34a1486", "fcfbfdefedf5dadaebbcbddc9e9ac8807dba6a51a354278f3f007d").map(colors_default);
var Purples_default = ramp_default(scheme25);
// node_modules/d3-scale-chromatic/src/sequential-single/Reds.js
var scheme26 = new Array(3).concat("fee0d2fc9272de2d26", "fee5d9fcae91fb6a4acb181d", "fee5d9fcae91fb6a4ade2d26a50f15", "fee5d9fcbba1fc9272fb6a4ade2d26a50f15", "fee5d9fcbba1fc9272fb6a4aef3b2ccb181d99000d", "fff5f0fee0d2fcbba1fc9272fb6a4aef3b2ccb181d99000d", "fff5f0fee0d2fcbba1fc9272fb6a4aef3b2ccb181da50f1567000d").map(colors_default);
var Reds_default = ramp_default(scheme26);
// node_modules/d3-scale-chromatic/src/sequential-single/Oranges.js
var scheme27 = new Array(3).concat("fee6cefdae6be6550d", "feeddefdbe85fd8d3cd94701", "feeddefdbe85fd8d3ce6550da63603", "feeddefdd0a2fdae6bfd8d3ce6550da63603", "feeddefdd0a2fdae6bfd8d3cf16913d948018c2d04", "fff5ebfee6cefdd0a2fdae6bfd8d3cf16913d948018c2d04", "fff5ebfee6cefdd0a2fdae6bfd8d3cf16913d94801a636037f2704").map(colors_default);
var Oranges_default = ramp_default(scheme27);
// node_modules/d3-scale-chromatic/src/sequential-multi/cividis.js
function cividis_default(t) {
  t = Math.max(0, Math.min(1, t));
  return "rgb(" + Math.max(0, Math.min(255, Math.round(-4.54 - t * (35.34 - t * (2381.73 - t * (6402.7 - t * (7024.72 - t * 2710.57))))))) + ", " + Math.max(0, Math.min(255, Math.round(32.49 + t * (170.73 + t * (52.82 - t * (131.46 - t * (176.58 - t * 67.37))))))) + ", " + Math.max(0, Math.min(255, Math.round(81.24 + t * (442.36 - t * (2482.43 - t * (6167.24 - t * (6614.94 - t * 2475.67))))))) + ")";
}
// node_modules/d3-scale-chromatic/src/sequential-multi/cubehelix.js
var cubehelix_default2 = cubehelixLong(cubehelix(300, 0.5, 0), cubehelix(-240, 0.5, 1));
// node_modules/d3-scale-chromatic/src/sequential-multi/rainbow.js
var warm = cubehelixLong(cubehelix(-100, 0.75, 0.35), cubehelix(80, 1.5, 0.8));
var cool = cubehelixLong(cubehelix(260, 0.75, 0.35), cubehelix(80, 1.5, 0.8));
var c3 = cubehelix();
function rainbow_default(t) {
  if (t < 0 || t > 1)
    t -= Math.floor(t);
  var ts = Math.abs(t - 0.5);
  c3.h = 360 * t - 100;
  c3.s = 1.5 - 1.5 * ts;
  c3.l = 0.8 - 0.9 * ts;
  return c3 + "";
}
// node_modules/d3-scale-chromatic/src/sequential-multi/sinebow.js
var c4 = rgb();
var pi_1_3 = Math.PI / 3;
var pi_2_3 = Math.PI * 2 / 3;
function sinebow_default(t) {
  var x3;
  t = (0.5 - t) * Math.PI;
  c4.r = 255 * (x3 = Math.sin(t)) * x3;
  c4.g = 255 * (x3 = Math.sin(t + pi_1_3)) * x3;
  c4.b = 255 * (x3 = Math.sin(t + pi_2_3)) * x3;
  return c4 + "";
}
// node_modules/d3-scale-chromatic/src/sequential-multi/turbo.js
function turbo_default(t) {
  t = Math.max(0, Math.min(1, t));
  return "rgb(" + Math.max(0, Math.min(255, Math.round(34.61 + t * (1172.33 - t * (10793.56 - t * (33300.12 - t * (38394.49 - t * 14825.05))))))) + ", " + Math.max(0, Math.min(255, Math.round(23.31 + t * (557.33 + t * (1225.33 - t * (3574.96 - t * (1073.77 + t * 707.56))))))) + ", " + Math.max(0, Math.min(255, Math.round(27.2 + t * (3211.1 - t * (15327.97 - t * (27814 - t * (22569.18 - t * 6838.66))))))) + ")";
}
// node_modules/d3-scale-chromatic/src/sequential-multi/viridis.js
function ramp(range4) {
  var n = range4.length;
  return function(t) {
    return range4[Math.max(0, Math.min(n - 1, Math.floor(t * n)))];
  };
}
var viridis_default = ramp(colors_default("44015444025645045745055946075a46085c460a5d460b5e470d60470e6147106347116447136548146748166848176948186a481a6c481b6d481c6e481d6f481f70482071482173482374482475482576482677482878482979472a7a472c7a472d7b472e7c472f7d46307e46327e46337f463480453581453781453882443983443a83443b84433d84433e85423f854240864241864142874144874045884046883f47883f48893e49893e4a893e4c8a3d4d8a3d4e8a3c4f8a3c508b3b518b3b528b3a538b3a548c39558c39568c38588c38598c375a8c375b8d365c8d365d8d355e8d355f8d34608d34618d33628d33638d32648e32658e31668e31678e31688e30698e306a8e2f6b8e2f6c8e2e6d8e2e6e8e2e6f8e2d708e2d718e2c718e2c728e2c738e2b748e2b758e2a768e2a778e2a788e29798e297a8e297b8e287c8e287d8e277e8e277f8e27808e26818e26828e26828e25838e25848e25858e24868e24878e23888e23898e238a8d228b8d228c8d228d8d218e8d218f8d21908d21918c20928c20928c20938c1f948c1f958b1f968b1f978b1f988b1f998a1f9a8a1e9b8a1e9c891e9d891f9e891f9f881fa0881fa1881fa1871fa28720a38620a48621a58521a68522a78522a88423a98324aa8325ab8225ac8226ad8127ad8128ae8029af7f2ab07f2cb17e2db27d2eb37c2fb47c31b57b32b67a34b67935b77937b87838b9773aba763bbb753dbc743fbc7340bd7242be7144bf7046c06f48c16e4ac16d4cc26c4ec36b50c46a52c56954c56856c66758c7655ac8645cc8635ec96260ca6063cb5f65cb5e67cc5c69cd5b6ccd5a6ece5870cf5773d05675d05477d1537ad1517cd2507fd34e81d34d84d44b86d54989d5488bd6468ed64590d74393d74195d84098d83e9bd93c9dd93ba0da39a2da37a5db36a8db34aadc32addc30b0dd2fb2dd2db5de2bb8de29bade28bddf26c0df25c2df23c5e021c8e020cae11fcde11dd0e11cd2e21bd5e21ad8e219dae319dde318dfe318e2e418e5e419e7e419eae51aece51befe51cf1e51df4e61ef6e620f8e621fbe723fde725"));
var magma = ramp(colors_default("00000401000501010601010802010902020b02020d03030f03031204041405041606051806051a07061c08071e0907200a08220b09240c09260d0a290e0b2b100b2d110c2f120d31130d34140e36150e38160f3b180f3d19103f1a10421c10441d11471e114920114b21114e22115024125325125527125829115a2a115c2c115f2d11612f116331116533106734106936106b38106c390f6e3b0f703d0f713f0f72400f74420f75440f764510774710784910784a10794c117a4e117b4f127b51127c52137c54137d56147d57157e59157e5a167e5c167f5d177f5f187f601880621980641a80651a80671b80681c816a1c816b1d816d1d816e1e81701f81721f817320817521817621817822817922827b23827c23827e24828025828125818326818426818627818827818928818b29818c29818e2a81902a81912b81932b80942c80962c80982d80992d809b2e7f9c2e7f9e2f7fa02f7fa1307ea3307ea5317ea6317da8327daa337dab337cad347cae347bb0357bb2357bb3367ab5367ab73779b83779ba3878bc3978bd3977bf3a77c03a76c23b75c43c75c53c74c73d73c83e73ca3e72cc3f71cd4071cf4070d0416fd2426fd3436ed5446dd6456cd8456cd9466bdb476adc4869de4968df4a68e04c67e24d66e34e65e44f64e55064e75263e85362e95462ea5661eb5760ec5860ed5a5fee5b5eef5d5ef05f5ef1605df2625df2645cf3655cf4675cf4695cf56b5cf66c5cf66e5cf7705cf7725cf8745cf8765cf9785df9795df97b5dfa7d5efa7f5efa815ffb835ffb8560fb8761fc8961fc8a62fc8c63fc8e64fc9065fd9266fd9467fd9668fd9869fd9a6afd9b6bfe9d6cfe9f6dfea16efea36ffea571fea772fea973feaa74feac76feae77feb078feb27afeb47bfeb67cfeb77efeb97ffebb81febd82febf84fec185fec287fec488fec68afec88cfeca8dfecc8ffecd90fecf92fed194fed395fed597fed799fed89afdda9cfddc9efddea0fde0a1fde2a3fde3a5fde5a7fde7a9fde9aafdebacfcecaefceeb0fcf0b2fcf2b4fcf4b6fcf6b8fcf7b9fcf9bbfcfbbdfcfdbf"));
var inferno = ramp(colors_default("00000401000501010601010802010a02020c02020e03021004031204031405041706041907051b08051d09061f0a07220b07240c08260d08290e092b10092d110a30120a32140b34150b37160b39180c3c190c3e1b0c411c0c431e0c451f0c48210c4a230c4c240c4f260c51280b53290b552b0b572d0b592f0a5b310a5c320a5e340a5f3609613809623909633b09643d09653e0966400a67420a68440a68450a69470b6a490b6a4a0c6b4c0c6b4d0d6c4f0d6c510e6c520e6d540f6d550f6d57106e59106e5a116e5c126e5d126e5f136e61136e62146e64156e65156e67166e69166e6a176e6c186e6d186e6f196e71196e721a6e741a6e751b6e771c6d781c6d7a1d6d7c1d6d7d1e6d7f1e6c801f6c82206c84206b85216b87216b88226a8a226a8c23698d23698f24699025689225689326679526679727669827669a28659b29649d29649f2a63a02a63a22b62a32c61a52c60a62d60a82e5fa92e5eab2f5ead305dae305cb0315bb1325ab3325ab43359b63458b73557b93556ba3655bc3754bd3853bf3952c03a51c13a50c33b4fc43c4ec63d4dc73e4cc83f4bca404acb4149cc4248ce4347cf4446d04545d24644d34743d44842d54a41d74b3fd84c3ed94d3dda4e3cdb503bdd513ade5238df5337e05536e15635e25734e35933e45a31e55c30e65d2fe75e2ee8602de9612bea632aeb6429eb6628ec6726ed6925ee6a24ef6c23ef6e21f06f20f1711ff1731df2741cf3761bf37819f47918f57b17f57d15f67e14f68013f78212f78410f8850ff8870ef8890cf98b0bf98c0af98e09fa9008fa9207fa9407fb9606fb9706fb9906fb9b06fb9d07fc9f07fca108fca309fca50afca60cfca80dfcaa0ffcac11fcae12fcb014fcb216fcb418fbb61afbb81dfbba1ffbbc21fbbe23fac026fac228fac42afac62df9c72ff9c932f9cb35f8cd37f8cf3af7d13df7d340f6d543f6d746f5d949f5db4cf4dd4ff4df53f4e156f3e35af3e55df2e661f2e865f2ea69f1ec6df1ed71f1ef75f1f179f2f27df2f482f3f586f3f68af4f88ef5f992f6fa96f8fb9af9fc9dfafda1fcffa4"));
var plasma = ramp(colors_default("0d088710078813078916078a19068c1b068d1d068e20068f2206902406912605912805922a05932c05942e05952f059631059733059735049837049938049a3a049a3c049b3e049c3f049c41049d43039e44039e46039f48039f4903a04b03a14c02a14e02a25002a25102a35302a35502a45601a45801a45901a55b01a55c01a65e01a66001a66100a76300a76400a76600a76700a86900a86a00a86c00a86e00a86f00a87100a87201a87401a87501a87701a87801a87a02a87b02a87d03a87e03a88004a88104a78305a78405a78606a68707a68808a68a09a58b0aa58d0ba58e0ca48f0da4910ea3920fa39410a29511a19613a19814a099159f9a169f9c179e9d189d9e199da01a9ca11b9ba21d9aa31e9aa51f99a62098a72197a82296aa2395ab2494ac2694ad2793ae2892b02991b12a90b22b8fb32c8eb42e8db52f8cb6308bb7318ab83289ba3388bb3488bc3587bd3786be3885bf3984c03a83c13b82c23c81c33d80c43e7fc5407ec6417dc7427cc8437bc9447aca457acb4679cc4778cc4977cd4a76ce4b75cf4c74d04d73d14e72d24f71d35171d45270d5536fd5546ed6556dd7566cd8576bd9586ada5a6ada5b69db5c68dc5d67dd5e66de5f65de6164df6263e06363e16462e26561e26660e3685fe4695ee56a5de56b5de66c5ce76e5be76f5ae87059e97158e97257ea7457eb7556eb7655ec7754ed7953ed7a52ee7b51ef7c51ef7e50f07f4ff0804ef1814df1834cf2844bf3854bf3874af48849f48948f58b47f58c46f68d45f68f44f79044f79143f79342f89441f89540f9973ff9983ef99a3efa9b3dfa9c3cfa9e3bfb9f3afba139fba238fca338fca537fca636fca835fca934fdab33fdac33fdae32fdaf31fdb130fdb22ffdb42ffdb52efeb72dfeb82cfeba2cfebb2bfebd2afebe2afec029fdc229fdc328fdc527fdc627fdc827fdca26fdcb26fccd25fcce25fcd025fcd225fbd324fbd524fbd724fad824fada24f9dc24f9dd25f8df25f8e125f7e225f7e425f6e626f6e826f5e926f5eb27f4ed27f3ee27f3f027f2f227f1f426f1f525f0f724f0f921"));
// node_modules/d3-shape/src/constant.js
function constant_default10(x3) {
  return function constant2() {
    return x3;
  };
}

// node_modules/d3-shape/src/math.js
var abs4 = Math.abs;
var atan22 = Math.atan2;
var cos3 = Math.cos;
var max4 = Math.max;
var min3 = Math.min;
var sin3 = Math.sin;
var sqrt3 = Math.sqrt;
var epsilon9 = 0.000000000001;
var pi5 = Math.PI;
var halfPi4 = pi5 / 2;
var tau6 = 2 * pi5;
function acos2(x3) {
  return x3 > 1 ? 0 : x3 < -1 ? pi5 : Math.acos(x3);
}
function asin2(x3) {
  return x3 >= 1 ? halfPi4 : x3 <= -1 ? -halfPi4 : Math.asin(x3);
}

// node_modules/d3-shape/src/path.js
function withPath(shape) {
  let digits = 3;
  shape.digits = function(_) {
    if (!arguments.length)
      return digits;
    if (_ == null) {
      digits = null;
    } else {
      const d = Math.floor(_);
      if (!(d >= 0))
        throw new RangeError(`invalid digits: ${_}`);
      digits = d;
    }
    return shape;
  };
  return () => new Path(digits);
}

// node_modules/d3-shape/src/arc.js
function arcInnerRadius(d) {
  return d.innerRadius;
}
function arcOuterRadius(d) {
  return d.outerRadius;
}
function arcStartAngle(d) {
  return d.startAngle;
}
function arcEndAngle(d) {
  return d.endAngle;
}
function arcPadAngle(d) {
  return d && d.padAngle;
}
function intersect(x06, y06, x12, y12, x22, y22, x3, y3) {
  var x10 = x12 - x06, y10 = y12 - y06, x32 = x3 - x22, y32 = y3 - y22, t = y32 * x10 - x32 * y10;
  if (t * t < epsilon9)
    return;
  t = (x32 * (y06 - y22) - y32 * (x06 - x22)) / t;
  return [x06 + t * x10, y06 + t * y10];
}
function cornerTangents(x06, y06, x12, y12, r1, rc, cw) {
  var x01 = x06 - x12, y01 = y06 - y12, lo = (cw ? rc : -rc) / sqrt3(x01 * x01 + y01 * y01), ox = lo * y01, oy = -lo * x01, x11 = x06 + ox, y11 = y06 + oy, x10 = x12 + ox, y10 = y12 + oy, x004 = (x11 + x10) / 2, y004 = (y11 + y10) / 2, dx = x10 - x11, dy = y10 - y11, d2 = dx * dx + dy * dy, r = r1 - rc, D3 = x11 * y10 - x10 * y11, d = (dy < 0 ? -1 : 1) * sqrt3(max4(0, r * r * d2 - D3 * D3)), cx0 = (D3 * dy - dx * d) / d2, cy0 = (-D3 * dx - dy * d) / d2, cx1 = (D3 * dy + dx * d) / d2, cy1 = (-D3 * dx + dy * d) / d2, dx0 = cx0 - x004, dy0 = cy0 - y004, dx1 = cx1 - x004, dy1 = cy1 - y004;
  if (dx0 * dx0 + dy0 * dy0 > dx1 * dx1 + dy1 * dy1)
    cx0 = cx1, cy0 = cy1;
  return {
    cx: cx0,
    cy: cy0,
    x01: -ox,
    y01: -oy,
    x11: cx0 * (r1 / r - 1),
    y11: cy0 * (r1 / r - 1)
  };
}
function arc_default() {
  var innerRadius = arcInnerRadius, outerRadius = arcOuterRadius, cornerRadius = constant_default10(0), padRadius = null, startAngle = arcStartAngle, endAngle = arcEndAngle, padAngle = arcPadAngle, context = null, path2 = withPath(arc);
  function arc() {
    var buffer, r, r0 = +innerRadius.apply(this, arguments), r1 = +outerRadius.apply(this, arguments), a0 = startAngle.apply(this, arguments) - halfPi4, a1 = endAngle.apply(this, arguments) - halfPi4, da2 = abs4(a1 - a0), cw = a1 > a0;
    if (!context)
      context = buffer = path2();
    if (r1 < r0)
      r = r1, r1 = r0, r0 = r;
    if (!(r1 > epsilon9))
      context.moveTo(0, 0);
    else if (da2 > tau6 - epsilon9) {
      context.moveTo(r1 * cos3(a0), r1 * sin3(a0));
      context.arc(0, 0, r1, a0, a1, !cw);
      if (r0 > epsilon9) {
        context.moveTo(r0 * cos3(a1), r0 * sin3(a1));
        context.arc(0, 0, r0, a1, a0, cw);
      }
    } else {
      var a01 = a0, a11 = a1, a00 = a0, a10 = a1, da0 = da2, da1 = da2, ap = padAngle.apply(this, arguments) / 2, rp = ap > epsilon9 && (padRadius ? +padRadius.apply(this, arguments) : sqrt3(r0 * r0 + r1 * r1)), rc = min3(abs4(r1 - r0) / 2, +cornerRadius.apply(this, arguments)), rc0 = rc, rc1 = rc, t03, t13;
      if (rp > epsilon9) {
        var p02 = asin2(rp / r0 * sin3(ap)), p1 = asin2(rp / r1 * sin3(ap));
        if ((da0 -= p02 * 2) > epsilon9)
          p02 *= cw ? 1 : -1, a00 += p02, a10 -= p02;
        else
          da0 = 0, a00 = a10 = (a0 + a1) / 2;
        if ((da1 -= p1 * 2) > epsilon9)
          p1 *= cw ? 1 : -1, a01 += p1, a11 -= p1;
        else
          da1 = 0, a01 = a11 = (a0 + a1) / 2;
      }
      var x01 = r1 * cos3(a01), y01 = r1 * sin3(a01), x10 = r0 * cos3(a10), y10 = r0 * sin3(a10);
      if (rc > epsilon9) {
        var x11 = r1 * cos3(a11), y11 = r1 * sin3(a11), x004 = r0 * cos3(a00), y004 = r0 * sin3(a00), oc;
        if (da2 < pi5) {
          if (oc = intersect(x01, y01, x004, y004, x11, y11, x10, y10)) {
            var ax = x01 - oc[0], ay = y01 - oc[1], bx = x11 - oc[0], by = y11 - oc[1], kc = 1 / sin3(acos2((ax * bx + ay * by) / (sqrt3(ax * ax + ay * ay) * sqrt3(bx * bx + by * by))) / 2), lc = sqrt3(oc[0] * oc[0] + oc[1] * oc[1]);
            rc0 = min3(rc, (r0 - lc) / (kc - 1));
            rc1 = min3(rc, (r1 - lc) / (kc + 1));
          } else {
            rc0 = rc1 = 0;
          }
        }
      }
      if (!(da1 > epsilon9))
        context.moveTo(x01, y01);
      else if (rc1 > epsilon9) {
        t03 = cornerTangents(x004, y004, x01, y01, r1, rc1, cw);
        t13 = cornerTangents(x11, y11, x10, y10, r1, rc1, cw);
        context.moveTo(t03.cx + t03.x01, t03.cy + t03.y01);
        if (rc1 < rc)
          context.arc(t03.cx, t03.cy, rc1, atan22(t03.y01, t03.x01), atan22(t13.y01, t13.x01), !cw);
        else {
          context.arc(t03.cx, t03.cy, rc1, atan22(t03.y01, t03.x01), atan22(t03.y11, t03.x11), !cw);
          context.arc(0, 0, r1, atan22(t03.cy + t03.y11, t03.cx + t03.x11), atan22(t13.cy + t13.y11, t13.cx + t13.x11), !cw);
          context.arc(t13.cx, t13.cy, rc1, atan22(t13.y11, t13.x11), atan22(t13.y01, t13.x01), !cw);
        }
      } else
        context.moveTo(x01, y01), context.arc(0, 0, r1, a01, a11, !cw);
      if (!(r0 > epsilon9) || !(da0 > epsilon9))
        context.lineTo(x10, y10);
      else if (rc0 > epsilon9) {
        t03 = cornerTangents(x10, y10, x11, y11, r0, -rc0, cw);
        t13 = cornerTangents(x01, y01, x004, y004, r0, -rc0, cw);
        context.lineTo(t03.cx + t03.x01, t03.cy + t03.y01);
        if (rc0 < rc)
          context.arc(t03.cx, t03.cy, rc0, atan22(t03.y01, t03.x01), atan22(t13.y01, t13.x01), !cw);
        else {
          context.arc(t03.cx, t03.cy, rc0, atan22(t03.y01, t03.x01), atan22(t03.y11, t03.x11), !cw);
          context.arc(0, 0, r0, atan22(t03.cy + t03.y11, t03.cx + t03.x11), atan22(t13.cy + t13.y11, t13.cx + t13.x11), cw);
          context.arc(t13.cx, t13.cy, rc0, atan22(t13.y11, t13.x11), atan22(t13.y01, t13.x01), !cw);
        }
      } else
        context.arc(0, 0, r0, a10, a00, cw);
    }
    context.closePath();
    if (buffer)
      return context = null, buffer + "" || null;
  }
  arc.centroid = function() {
    var r = (+innerRadius.apply(this, arguments) + +outerRadius.apply(this, arguments)) / 2, a3 = (+startAngle.apply(this, arguments) + +endAngle.apply(this, arguments)) / 2 - pi5 / 2;
    return [cos3(a3) * r, sin3(a3) * r];
  };
  arc.innerRadius = function(_) {
    return arguments.length ? (innerRadius = typeof _ === "function" ? _ : constant_default10(+_), arc) : innerRadius;
  };
  arc.outerRadius = function(_) {
    return arguments.length ? (outerRadius = typeof _ === "function" ? _ : constant_default10(+_), arc) : outerRadius;
  };
  arc.cornerRadius = function(_) {
    return arguments.length ? (cornerRadius = typeof _ === "function" ? _ : constant_default10(+_), arc) : cornerRadius;
  };
  arc.padRadius = function(_) {
    return arguments.length ? (padRadius = _ == null ? null : typeof _ === "function" ? _ : constant_default10(+_), arc) : padRadius;
  };
  arc.startAngle = function(_) {
    return arguments.length ? (startAngle = typeof _ === "function" ? _ : constant_default10(+_), arc) : startAngle;
  };
  arc.endAngle = function(_) {
    return arguments.length ? (endAngle = typeof _ === "function" ? _ : constant_default10(+_), arc) : endAngle;
  };
  arc.padAngle = function(_) {
    return arguments.length ? (padAngle = typeof _ === "function" ? _ : constant_default10(+_), arc) : padAngle;
  };
  arc.context = function(_) {
    return arguments.length ? (context = _ == null ? null : _, arc) : context;
  };
  return arc;
}
// node_modules/d3-shape/src/array.js
var slice4 = Array.prototype.slice;
function array_default3(x3) {
  return typeof x3 === "object" && "length" in x3 ? x3 : Array.from(x3);
}

// node_modules/d3-shape/src/curve/linear.js
function Linear(context) {
  this._context = context;
}
Linear.prototype = {
  areaStart: function() {
    this._line = 0;
  },
  areaEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._point = 0;
  },
  lineEnd: function() {
    if (this._line || this._line !== 0 && this._point === 1)
      this._context.closePath();
    this._line = 1 - this._line;
  },
  point: function(x3, y3) {
    x3 = +x3, y3 = +y3;
    switch (this._point) {
      case 0:
        this._point = 1;
        this._line ? this._context.lineTo(x3, y3) : this._context.moveTo(x3, y3);
        break;
      case 1:
        this._point = 2;
      default:
        this._context.lineTo(x3, y3);
        break;
    }
  }
};
function linear_default(context) {
  return new Linear(context);
}

// node_modules/d3-shape/src/point.js
function x3(p) {
  return p[0];
}
function y3(p) {
  return p[1];
}

// node_modules/d3-shape/src/line.js
function line_default2(x4, y4) {
  var defined = constant_default10(true), context = null, curve = linear_default, output = null, path2 = withPath(line);
  x4 = typeof x4 === "function" ? x4 : x4 === undefined ? x3 : constant_default10(x4);
  y4 = typeof y4 === "function" ? y4 : y4 === undefined ? y3 : constant_default10(y4);
  function line(data) {
    var i, n = (data = array_default3(data)).length, d, defined0 = false, buffer;
    if (context == null)
      output = curve(buffer = path2());
    for (i = 0;i <= n; ++i) {
      if (!(i < n && defined(d = data[i], i, data)) === defined0) {
        if (defined0 = !defined0)
          output.lineStart();
        else
          output.lineEnd();
      }
      if (defined0)
        output.point(+x4(d, i, data), +y4(d, i, data));
    }
    if (buffer)
      return output = null, buffer + "" || null;
  }
  line.x = function(_) {
    return arguments.length ? (x4 = typeof _ === "function" ? _ : constant_default10(+_), line) : x4;
  };
  line.y = function(_) {
    return arguments.length ? (y4 = typeof _ === "function" ? _ : constant_default10(+_), line) : y4;
  };
  line.defined = function(_) {
    return arguments.length ? (defined = typeof _ === "function" ? _ : constant_default10(!!_), line) : defined;
  };
  line.curve = function(_) {
    return arguments.length ? (curve = _, context != null && (output = curve(context)), line) : curve;
  };
  line.context = function(_) {
    return arguments.length ? (_ == null ? context = output = null : output = curve(context = _), line) : context;
  };
  return line;
}

// node_modules/d3-shape/src/area.js
function area_default5(x06, y06, y12) {
  var x12 = null, defined = constant_default10(true), context = null, curve = linear_default, output = null, path2 = withPath(area);
  x06 = typeof x06 === "function" ? x06 : x06 === undefined ? x3 : constant_default10(+x06);
  y06 = typeof y06 === "function" ? y06 : y06 === undefined ? constant_default10(0) : constant_default10(+y06);
  y12 = typeof y12 === "function" ? y12 : y12 === undefined ? y3 : constant_default10(+y12);
  function area(data) {
    var i, j, k, n = (data = array_default3(data)).length, d, defined0 = false, buffer, x0z = new Array(n), y0z = new Array(n);
    if (context == null)
      output = curve(buffer = path2());
    for (i = 0;i <= n; ++i) {
      if (!(i < n && defined(d = data[i], i, data)) === defined0) {
        if (defined0 = !defined0) {
          j = i;
          output.areaStart();
          output.lineStart();
        } else {
          output.lineEnd();
          output.lineStart();
          for (k = i - 1;k >= j; --k) {
            output.point(x0z[k], y0z[k]);
          }
          output.lineEnd();
          output.areaEnd();
        }
      }
      if (defined0) {
        x0z[i] = +x06(d, i, data), y0z[i] = +y06(d, i, data);
        output.point(x12 ? +x12(d, i, data) : x0z[i], y12 ? +y12(d, i, data) : y0z[i]);
      }
    }
    if (buffer)
      return output = null, buffer + "" || null;
  }
  function arealine() {
    return line_default2().defined(defined).curve(curve).context(context);
  }
  area.x = function(_) {
    return arguments.length ? (x06 = typeof _ === "function" ? _ : constant_default10(+_), x12 = null, area) : x06;
  };
  area.x0 = function(_) {
    return arguments.length ? (x06 = typeof _ === "function" ? _ : constant_default10(+_), area) : x06;
  };
  area.x1 = function(_) {
    return arguments.length ? (x12 = _ == null ? null : typeof _ === "function" ? _ : constant_default10(+_), area) : x12;
  };
  area.y = function(_) {
    return arguments.length ? (y06 = typeof _ === "function" ? _ : constant_default10(+_), y12 = null, area) : y06;
  };
  area.y0 = function(_) {
    return arguments.length ? (y06 = typeof _ === "function" ? _ : constant_default10(+_), area) : y06;
  };
  area.y1 = function(_) {
    return arguments.length ? (y12 = _ == null ? null : typeof _ === "function" ? _ : constant_default10(+_), area) : y12;
  };
  area.lineX0 = area.lineY0 = function() {
    return arealine().x(x06).y(y06);
  };
  area.lineY1 = function() {
    return arealine().x(x06).y(y12);
  };
  area.lineX1 = function() {
    return arealine().x(x12).y(y06);
  };
  area.defined = function(_) {
    return arguments.length ? (defined = typeof _ === "function" ? _ : constant_default10(!!_), area) : defined;
  };
  area.curve = function(_) {
    return arguments.length ? (curve = _, context != null && (output = curve(context)), area) : curve;
  };
  area.context = function(_) {
    return arguments.length ? (_ == null ? context = output = null : output = curve(context = _), area) : context;
  };
  return area;
}
// node_modules/d3-shape/src/descending.js
function descending_default(a3, b) {
  return b < a3 ? -1 : b > a3 ? 1 : b >= a3 ? 0 : NaN;
}

// node_modules/d3-shape/src/identity.js
function identity_default5(d) {
  return d;
}

// node_modules/d3-shape/src/pie.js
function pie_default() {
  var value = identity_default5, sortValues = descending_default, sort2 = null, startAngle = constant_default10(0), endAngle = constant_default10(tau6), padAngle = constant_default10(0);
  function pie(data) {
    var i, n = (data = array_default3(data)).length, j, k, sum3 = 0, index3 = new Array(n), arcs = new Array(n), a0 = +startAngle.apply(this, arguments), da2 = Math.min(tau6, Math.max(-tau6, endAngle.apply(this, arguments) - a0)), a1, p = Math.min(Math.abs(da2) / n, padAngle.apply(this, arguments)), pa = p * (da2 < 0 ? -1 : 1), v2;
    for (i = 0;i < n; ++i) {
      if ((v2 = arcs[index3[i] = i] = +value(data[i], i, data)) > 0) {
        sum3 += v2;
      }
    }
    if (sortValues != null)
      index3.sort(function(i2, j2) {
        return sortValues(arcs[i2], arcs[j2]);
      });
    else if (sort2 != null)
      index3.sort(function(i2, j2) {
        return sort2(data[i2], data[j2]);
      });
    for (i = 0, k = sum3 ? (da2 - n * pa) / sum3 : 0;i < n; ++i, a0 = a1) {
      j = index3[i], v2 = arcs[j], a1 = a0 + (v2 > 0 ? v2 * k : 0) + pa, arcs[j] = {
        data: data[j],
        index: i,
        value: v2,
        startAngle: a0,
        endAngle: a1,
        padAngle: p
      };
    }
    return arcs;
  }
  pie.value = function(_) {
    return arguments.length ? (value = typeof _ === "function" ? _ : constant_default10(+_), pie) : value;
  };
  pie.sortValues = function(_) {
    return arguments.length ? (sortValues = _, sort2 = null, pie) : sortValues;
  };
  pie.sort = function(_) {
    return arguments.length ? (sort2 = _, sortValues = null, pie) : sort2;
  };
  pie.startAngle = function(_) {
    return arguments.length ? (startAngle = typeof _ === "function" ? _ : constant_default10(+_), pie) : startAngle;
  };
  pie.endAngle = function(_) {
    return arguments.length ? (endAngle = typeof _ === "function" ? _ : constant_default10(+_), pie) : endAngle;
  };
  pie.padAngle = function(_) {
    return arguments.length ? (padAngle = typeof _ === "function" ? _ : constant_default10(+_), pie) : padAngle;
  };
  return pie;
}
// node_modules/d3-shape/src/curve/radial.js
var curveRadialLinear = curveRadial(linear_default);
function Radial(curve) {
  this._curve = curve;
}
Radial.prototype = {
  areaStart: function() {
    this._curve.areaStart();
  },
  areaEnd: function() {
    this._curve.areaEnd();
  },
  lineStart: function() {
    this._curve.lineStart();
  },
  lineEnd: function() {
    this._curve.lineEnd();
  },
  point: function(a3, r) {
    this._curve.point(r * Math.sin(a3), r * -Math.cos(a3));
  }
};
function curveRadial(curve) {
  function radial2(context) {
    return new Radial(curve(context));
  }
  radial2._curve = curve;
  return radial2;
}

// node_modules/d3-shape/src/lineRadial.js
function lineRadial(l) {
  var c5 = l.curve;
  l.angle = l.x, delete l.x;
  l.radius = l.y, delete l.y;
  l.curve = function(_) {
    return arguments.length ? c5(curveRadial(_)) : c5()._curve;
  };
  return l;
}
function lineRadial_default() {
  return lineRadial(line_default2().curve(curveRadialLinear));
}

// node_modules/d3-shape/src/areaRadial.js
function areaRadial_default() {
  var a3 = area_default5().curve(curveRadialLinear), c5 = a3.curve, x06 = a3.lineX0, x12 = a3.lineX1, y06 = a3.lineY0, y12 = a3.lineY1;
  a3.angle = a3.x, delete a3.x;
  a3.startAngle = a3.x0, delete a3.x0;
  a3.endAngle = a3.x1, delete a3.x1;
  a3.radius = a3.y, delete a3.y;
  a3.innerRadius = a3.y0, delete a3.y0;
  a3.outerRadius = a3.y1, delete a3.y1;
  a3.lineStartAngle = function() {
    return lineRadial(x06());
  }, delete a3.lineX0;
  a3.lineEndAngle = function() {
    return lineRadial(x12());
  }, delete a3.lineX1;
  a3.lineInnerRadius = function() {
    return lineRadial(y06());
  }, delete a3.lineY0;
  a3.lineOuterRadius = function() {
    return lineRadial(y12());
  }, delete a3.lineY1;
  a3.curve = function(_) {
    return arguments.length ? c5(curveRadial(_)) : c5()._curve;
  };
  return a3;
}
// node_modules/d3-shape/src/pointRadial.js
function pointRadial_default(x4, y4) {
  return [(y4 = +y4) * Math.cos(x4 -= Math.PI / 2), y4 * Math.sin(x4)];
}
// node_modules/d3-shape/src/curve/bump.js
class Bump {
  constructor(context, x4) {
    this._context = context;
    this._x = x4;
  }
  areaStart() {
    this._line = 0;
  }
  areaEnd() {
    this._line = NaN;
  }
  lineStart() {
    this._point = 0;
  }
  lineEnd() {
    if (this._line || this._line !== 0 && this._point === 1)
      this._context.closePath();
    this._line = 1 - this._line;
  }
  point(x4, y4) {
    x4 = +x4, y4 = +y4;
    switch (this._point) {
      case 0: {
        this._point = 1;
        if (this._line)
          this._context.lineTo(x4, y4);
        else
          this._context.moveTo(x4, y4);
        break;
      }
      case 1:
        this._point = 2;
      default: {
        if (this._x)
          this._context.bezierCurveTo(this._x0 = (this._x0 + x4) / 2, this._y0, this._x0, y4, x4, y4);
        else
          this._context.bezierCurveTo(this._x0, this._y0 = (this._y0 + y4) / 2, x4, this._y0, x4, y4);
        break;
      }
    }
    this._x0 = x4, this._y0 = y4;
  }
}

class BumpRadial {
  constructor(context) {
    this._context = context;
  }
  lineStart() {
    this._point = 0;
  }
  lineEnd() {}
  point(x4, y4) {
    x4 = +x4, y4 = +y4;
    if (this._point === 0) {
      this._point = 1;
    } else {
      const p02 = pointRadial_default(this._x0, this._y0);
      const p1 = pointRadial_default(this._x0, this._y0 = (this._y0 + y4) / 2);
      const p2 = pointRadial_default(x4, this._y0);
      const p3 = pointRadial_default(x4, y4);
      this._context.moveTo(...p02);
      this._context.bezierCurveTo(...p1, ...p2, ...p3);
    }
    this._x0 = x4, this._y0 = y4;
  }
}
function bumpX(context) {
  return new Bump(context, true);
}
function bumpY(context) {
  return new Bump(context, false);
}
function bumpRadial(context) {
  return new BumpRadial(context);
}

// node_modules/d3-shape/src/link.js
function linkSource(d) {
  return d.source;
}
function linkTarget(d) {
  return d.target;
}
function link2(curve) {
  let source = linkSource, target = linkTarget, x4 = x3, y4 = y3, context = null, output = null, path2 = withPath(link3);
  function link3() {
    let buffer;
    const argv = slice4.call(arguments);
    const s = source.apply(this, argv);
    const t = target.apply(this, argv);
    if (context == null)
      output = curve(buffer = path2());
    output.lineStart();
    argv[0] = s, output.point(+x4.apply(this, argv), +y4.apply(this, argv));
    argv[0] = t, output.point(+x4.apply(this, argv), +y4.apply(this, argv));
    output.lineEnd();
    if (buffer)
      return output = null, buffer + "" || null;
  }
  link3.source = function(_) {
    return arguments.length ? (source = _, link3) : source;
  };
  link3.target = function(_) {
    return arguments.length ? (target = _, link3) : target;
  };
  link3.x = function(_) {
    return arguments.length ? (x4 = typeof _ === "function" ? _ : constant_default10(+_), link3) : x4;
  };
  link3.y = function(_) {
    return arguments.length ? (y4 = typeof _ === "function" ? _ : constant_default10(+_), link3) : y4;
  };
  link3.context = function(_) {
    return arguments.length ? (_ == null ? context = output = null : output = curve(context = _), link3) : context;
  };
  return link3;
}
function linkHorizontal() {
  return link2(bumpX);
}
function linkVertical() {
  return link2(bumpY);
}
function linkRadial() {
  const l = link2(bumpRadial);
  l.angle = l.x, delete l.x;
  l.radius = l.y, delete l.y;
  return l;
}
// node_modules/d3-shape/src/symbol/asterisk.js
var sqrt32 = sqrt3(3);
var asterisk_default = {
  draw(context, size) {
    const r = sqrt3(size + min3(size / 28, 0.75)) * 0.59436;
    const t = r / 2;
    const u4 = t * sqrt32;
    context.moveTo(0, r);
    context.lineTo(0, -r);
    context.moveTo(-u4, -t);
    context.lineTo(u4, t);
    context.moveTo(-u4, t);
    context.lineTo(u4, -t);
  }
};

// node_modules/d3-shape/src/symbol/circle.js
var circle_default3 = {
  draw(context, size) {
    const r = sqrt3(size / pi5);
    context.moveTo(r, 0);
    context.arc(0, 0, r, 0, tau6);
  }
};

// node_modules/d3-shape/src/symbol/cross.js
var cross_default2 = {
  draw(context, size) {
    const r = sqrt3(size / 5) / 2;
    context.moveTo(-3 * r, -r);
    context.lineTo(-r, -r);
    context.lineTo(-r, -3 * r);
    context.lineTo(r, -3 * r);
    context.lineTo(r, -r);
    context.lineTo(3 * r, -r);
    context.lineTo(3 * r, r);
    context.lineTo(r, r);
    context.lineTo(r, 3 * r);
    context.lineTo(-r, 3 * r);
    context.lineTo(-r, r);
    context.lineTo(-3 * r, r);
    context.closePath();
  }
};

// node_modules/d3-shape/src/symbol/diamond.js
var tan30 = sqrt3(1 / 3);
var tan30_2 = tan30 * 2;
var diamond_default = {
  draw(context, size) {
    const y4 = sqrt3(size / tan30_2);
    const x4 = y4 * tan30;
    context.moveTo(0, -y4);
    context.lineTo(x4, 0);
    context.lineTo(0, y4);
    context.lineTo(-x4, 0);
    context.closePath();
  }
};

// node_modules/d3-shape/src/symbol/diamond2.js
var diamond2_default = {
  draw(context, size) {
    const r = sqrt3(size) * 0.62625;
    context.moveTo(0, -r);
    context.lineTo(r, 0);
    context.lineTo(0, r);
    context.lineTo(-r, 0);
    context.closePath();
  }
};

// node_modules/d3-shape/src/symbol/plus.js
var plus_default = {
  draw(context, size) {
    const r = sqrt3(size - min3(size / 7, 2)) * 0.87559;
    context.moveTo(-r, 0);
    context.lineTo(r, 0);
    context.moveTo(0, r);
    context.lineTo(0, -r);
  }
};

// node_modules/d3-shape/src/symbol/square.js
var square_default = {
  draw(context, size) {
    const w = sqrt3(size);
    const x4 = -w / 2;
    context.rect(x4, x4, w, w);
  }
};

// node_modules/d3-shape/src/symbol/square2.js
var square2_default = {
  draw(context, size) {
    const r = sqrt3(size) * 0.4431;
    context.moveTo(r, r);
    context.lineTo(r, -r);
    context.lineTo(-r, -r);
    context.lineTo(-r, r);
    context.closePath();
  }
};

// node_modules/d3-shape/src/symbol/star.js
var ka = 0.8908130915292852;
var kr = sin3(pi5 / 10) / sin3(7 * pi5 / 10);
var kx = sin3(tau6 / 10) * kr;
var ky = -cos3(tau6 / 10) * kr;
var star_default = {
  draw(context, size) {
    const r = sqrt3(size * ka);
    const x4 = kx * r;
    const y4 = ky * r;
    context.moveTo(0, -r);
    context.lineTo(x4, y4);
    for (let i = 1;i < 5; ++i) {
      const a3 = tau6 * i / 5;
      const c5 = cos3(a3);
      const s = sin3(a3);
      context.lineTo(s * r, -c5 * r);
      context.lineTo(c5 * x4 - s * y4, s * x4 + c5 * y4);
    }
    context.closePath();
  }
};

// node_modules/d3-shape/src/symbol/triangle.js
var sqrt33 = sqrt3(3);
var triangle_default = {
  draw(context, size) {
    const y4 = -sqrt3(size / (sqrt33 * 3));
    context.moveTo(0, y4 * 2);
    context.lineTo(-sqrt33 * y4, -y4);
    context.lineTo(sqrt33 * y4, -y4);
    context.closePath();
  }
};

// node_modules/d3-shape/src/symbol/triangle2.js
var sqrt34 = sqrt3(3);
var triangle2_default = {
  draw(context, size) {
    const s = sqrt3(size) * 0.6824;
    const t = s / 2;
    const u4 = s * sqrt34 / 2;
    context.moveTo(0, -s);
    context.lineTo(u4, t);
    context.lineTo(-u4, t);
    context.closePath();
  }
};

// node_modules/d3-shape/src/symbol/wye.js
var c5 = -0.5;
var s = sqrt3(3) / 2;
var k = 1 / sqrt3(12);
var a3 = (k / 2 + 1) * 3;
var wye_default = {
  draw(context, size) {
    const r = sqrt3(size / a3);
    const x06 = r / 2, y06 = r * k;
    const x12 = x06, y12 = r * k + r;
    const x22 = -x12, y22 = y12;
    context.moveTo(x06, y06);
    context.lineTo(x12, y12);
    context.lineTo(x22, y22);
    context.lineTo(c5 * x06 - s * y06, s * x06 + c5 * y06);
    context.lineTo(c5 * x12 - s * y12, s * x12 + c5 * y12);
    context.lineTo(c5 * x22 - s * y22, s * x22 + c5 * y22);
    context.lineTo(c5 * x06 + s * y06, c5 * y06 - s * x06);
    context.lineTo(c5 * x12 + s * y12, c5 * y12 - s * x12);
    context.lineTo(c5 * x22 + s * y22, c5 * y22 - s * x22);
    context.closePath();
  }
};

// node_modules/d3-shape/src/symbol/times.js
var times_default = {
  draw(context, size) {
    const r = sqrt3(size - min3(size / 6, 1.7)) * 0.6189;
    context.moveTo(-r, -r);
    context.lineTo(r, r);
    context.moveTo(-r, r);
    context.lineTo(r, -r);
  }
};

// node_modules/d3-shape/src/symbol.js
var symbolsFill = [
  circle_default3,
  cross_default2,
  diamond_default,
  square_default,
  star_default,
  triangle_default,
  wye_default
];
var symbolsStroke = [
  circle_default3,
  plus_default,
  times_default,
  triangle2_default,
  asterisk_default,
  square2_default,
  diamond2_default
];
function Symbol2(type2, size) {
  let context = null, path2 = withPath(symbol);
  type2 = typeof type2 === "function" ? type2 : constant_default10(type2 || circle_default3);
  size = typeof size === "function" ? size : constant_default10(size === undefined ? 64 : +size);
  function symbol() {
    let buffer;
    if (!context)
      context = buffer = path2();
    type2.apply(this, arguments).draw(context, +size.apply(this, arguments));
    if (buffer)
      return context = null, buffer + "" || null;
  }
  symbol.type = function(_) {
    return arguments.length ? (type2 = typeof _ === "function" ? _ : constant_default10(_), symbol) : type2;
  };
  symbol.size = function(_) {
    return arguments.length ? (size = typeof _ === "function" ? _ : constant_default10(+_), symbol) : size;
  };
  symbol.context = function(_) {
    return arguments.length ? (context = _ == null ? null : _, symbol) : context;
  };
  return symbol;
}
// node_modules/d3-shape/src/noop.js
function noop_default2() {}

// node_modules/d3-shape/src/curve/basis.js
function point2(that, x4, y4) {
  that._context.bezierCurveTo((2 * that._x0 + that._x1) / 3, (2 * that._y0 + that._y1) / 3, (that._x0 + 2 * that._x1) / 3, (that._y0 + 2 * that._y1) / 3, (that._x0 + 4 * that._x1 + x4) / 6, (that._y0 + 4 * that._y1 + y4) / 6);
}
function Basis(context) {
  this._context = context;
}
Basis.prototype = {
  areaStart: function() {
    this._line = 0;
  },
  areaEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._x0 = this._x1 = this._y0 = this._y1 = NaN;
    this._point = 0;
  },
  lineEnd: function() {
    switch (this._point) {
      case 3:
        point2(this, this._x1, this._y1);
      case 2:
        this._context.lineTo(this._x1, this._y1);
        break;
    }
    if (this._line || this._line !== 0 && this._point === 1)
      this._context.closePath();
    this._line = 1 - this._line;
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    switch (this._point) {
      case 0:
        this._point = 1;
        this._line ? this._context.lineTo(x4, y4) : this._context.moveTo(x4, y4);
        break;
      case 1:
        this._point = 2;
        break;
      case 2:
        this._point = 3;
        this._context.lineTo((5 * this._x0 + this._x1) / 6, (5 * this._y0 + this._y1) / 6);
      default:
        point2(this, x4, y4);
        break;
    }
    this._x0 = this._x1, this._x1 = x4;
    this._y0 = this._y1, this._y1 = y4;
  }
};
function basis_default2(context) {
  return new Basis(context);
}

// node_modules/d3-shape/src/curve/basisClosed.js
function BasisClosed(context) {
  this._context = context;
}
BasisClosed.prototype = {
  areaStart: noop_default2,
  areaEnd: noop_default2,
  lineStart: function() {
    this._x0 = this._x1 = this._x2 = this._x3 = this._x4 = this._y0 = this._y1 = this._y2 = this._y3 = this._y4 = NaN;
    this._point = 0;
  },
  lineEnd: function() {
    switch (this._point) {
      case 1: {
        this._context.moveTo(this._x2, this._y2);
        this._context.closePath();
        break;
      }
      case 2: {
        this._context.moveTo((this._x2 + 2 * this._x3) / 3, (this._y2 + 2 * this._y3) / 3);
        this._context.lineTo((this._x3 + 2 * this._x2) / 3, (this._y3 + 2 * this._y2) / 3);
        this._context.closePath();
        break;
      }
      case 3: {
        this.point(this._x2, this._y2);
        this.point(this._x3, this._y3);
        this.point(this._x4, this._y4);
        break;
      }
    }
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    switch (this._point) {
      case 0:
        this._point = 1;
        this._x2 = x4, this._y2 = y4;
        break;
      case 1:
        this._point = 2;
        this._x3 = x4, this._y3 = y4;
        break;
      case 2:
        this._point = 3;
        this._x4 = x4, this._y4 = y4;
        this._context.moveTo((this._x0 + 4 * this._x1 + x4) / 6, (this._y0 + 4 * this._y1 + y4) / 6);
        break;
      default:
        point2(this, x4, y4);
        break;
    }
    this._x0 = this._x1, this._x1 = x4;
    this._y0 = this._y1, this._y1 = y4;
  }
};
function basisClosed_default2(context) {
  return new BasisClosed(context);
}
// node_modules/d3-shape/src/curve/basisOpen.js
function BasisOpen(context) {
  this._context = context;
}
BasisOpen.prototype = {
  areaStart: function() {
    this._line = 0;
  },
  areaEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._x0 = this._x1 = this._y0 = this._y1 = NaN;
    this._point = 0;
  },
  lineEnd: function() {
    if (this._line || this._line !== 0 && this._point === 3)
      this._context.closePath();
    this._line = 1 - this._line;
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    switch (this._point) {
      case 0:
        this._point = 1;
        break;
      case 1:
        this._point = 2;
        break;
      case 2:
        this._point = 3;
        var x06 = (this._x0 + 4 * this._x1 + x4) / 6, y06 = (this._y0 + 4 * this._y1 + y4) / 6;
        this._line ? this._context.lineTo(x06, y06) : this._context.moveTo(x06, y06);
        break;
      case 3:
        this._point = 4;
      default:
        point2(this, x4, y4);
        break;
    }
    this._x0 = this._x1, this._x1 = x4;
    this._y0 = this._y1, this._y1 = y4;
  }
};
function basisOpen_default(context) {
  return new BasisOpen(context);
}
// node_modules/d3-shape/src/curve/bundle.js
function Bundle(context, beta) {
  this._basis = new Basis(context);
  this._beta = beta;
}
Bundle.prototype = {
  lineStart: function() {
    this._x = [];
    this._y = [];
    this._basis.lineStart();
  },
  lineEnd: function() {
    var x4 = this._x, y4 = this._y, j = x4.length - 1;
    if (j > 0) {
      var x06 = x4[0], y06 = y4[0], dx = x4[j] - x06, dy = y4[j] - y06, i = -1, t;
      while (++i <= j) {
        t = i / j;
        this._basis.point(this._beta * x4[i] + (1 - this._beta) * (x06 + t * dx), this._beta * y4[i] + (1 - this._beta) * (y06 + t * dy));
      }
    }
    this._x = this._y = null;
    this._basis.lineEnd();
  },
  point: function(x4, y4) {
    this._x.push(+x4);
    this._y.push(+y4);
  }
};
var bundle_default = function custom12(beta) {
  function bundle(context) {
    return beta === 1 ? new Basis(context) : new Bundle(context, beta);
  }
  bundle.beta = function(beta2) {
    return custom12(+beta2);
  };
  return bundle;
}(0.85);
// node_modules/d3-shape/src/curve/cardinal.js
function point3(that, x4, y4) {
  that._context.bezierCurveTo(that._x1 + that._k * (that._x2 - that._x0), that._y1 + that._k * (that._y2 - that._y0), that._x2 + that._k * (that._x1 - x4), that._y2 + that._k * (that._y1 - y4), that._x2, that._y2);
}
function Cardinal(context, tension) {
  this._context = context;
  this._k = (1 - tension) / 6;
}
Cardinal.prototype = {
  areaStart: function() {
    this._line = 0;
  },
  areaEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._x0 = this._x1 = this._x2 = this._y0 = this._y1 = this._y2 = NaN;
    this._point = 0;
  },
  lineEnd: function() {
    switch (this._point) {
      case 2:
        this._context.lineTo(this._x2, this._y2);
        break;
      case 3:
        point3(this, this._x1, this._y1);
        break;
    }
    if (this._line || this._line !== 0 && this._point === 1)
      this._context.closePath();
    this._line = 1 - this._line;
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    switch (this._point) {
      case 0:
        this._point = 1;
        this._line ? this._context.lineTo(x4, y4) : this._context.moveTo(x4, y4);
        break;
      case 1:
        this._point = 2;
        this._x1 = x4, this._y1 = y4;
        break;
      case 2:
        this._point = 3;
      default:
        point3(this, x4, y4);
        break;
    }
    this._x0 = this._x1, this._x1 = this._x2, this._x2 = x4;
    this._y0 = this._y1, this._y1 = this._y2, this._y2 = y4;
  }
};
var cardinal_default = function custom13(tension) {
  function cardinal(context) {
    return new Cardinal(context, tension);
  }
  cardinal.tension = function(tension2) {
    return custom13(+tension2);
  };
  return cardinal;
}(0);

// node_modules/d3-shape/src/curve/cardinalClosed.js
function CardinalClosed(context, tension) {
  this._context = context;
  this._k = (1 - tension) / 6;
}
CardinalClosed.prototype = {
  areaStart: noop_default2,
  areaEnd: noop_default2,
  lineStart: function() {
    this._x0 = this._x1 = this._x2 = this._x3 = this._x4 = this._x5 = this._y0 = this._y1 = this._y2 = this._y3 = this._y4 = this._y5 = NaN;
    this._point = 0;
  },
  lineEnd: function() {
    switch (this._point) {
      case 1: {
        this._context.moveTo(this._x3, this._y3);
        this._context.closePath();
        break;
      }
      case 2: {
        this._context.lineTo(this._x3, this._y3);
        this._context.closePath();
        break;
      }
      case 3: {
        this.point(this._x3, this._y3);
        this.point(this._x4, this._y4);
        this.point(this._x5, this._y5);
        break;
      }
    }
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    switch (this._point) {
      case 0:
        this._point = 1;
        this._x3 = x4, this._y3 = y4;
        break;
      case 1:
        this._point = 2;
        this._context.moveTo(this._x4 = x4, this._y4 = y4);
        break;
      case 2:
        this._point = 3;
        this._x5 = x4, this._y5 = y4;
        break;
      default:
        point3(this, x4, y4);
        break;
    }
    this._x0 = this._x1, this._x1 = this._x2, this._x2 = x4;
    this._y0 = this._y1, this._y1 = this._y2, this._y2 = y4;
  }
};
var cardinalClosed_default = function custom14(tension) {
  function cardinal(context) {
    return new CardinalClosed(context, tension);
  }
  cardinal.tension = function(tension2) {
    return custom14(+tension2);
  };
  return cardinal;
}(0);
// node_modules/d3-shape/src/curve/cardinalOpen.js
function CardinalOpen(context, tension) {
  this._context = context;
  this._k = (1 - tension) / 6;
}
CardinalOpen.prototype = {
  areaStart: function() {
    this._line = 0;
  },
  areaEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._x0 = this._x1 = this._x2 = this._y0 = this._y1 = this._y2 = NaN;
    this._point = 0;
  },
  lineEnd: function() {
    if (this._line || this._line !== 0 && this._point === 3)
      this._context.closePath();
    this._line = 1 - this._line;
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    switch (this._point) {
      case 0:
        this._point = 1;
        break;
      case 1:
        this._point = 2;
        break;
      case 2:
        this._point = 3;
        this._line ? this._context.lineTo(this._x2, this._y2) : this._context.moveTo(this._x2, this._y2);
        break;
      case 3:
        this._point = 4;
      default:
        point3(this, x4, y4);
        break;
    }
    this._x0 = this._x1, this._x1 = this._x2, this._x2 = x4;
    this._y0 = this._y1, this._y1 = this._y2, this._y2 = y4;
  }
};
var cardinalOpen_default = function custom15(tension) {
  function cardinal(context) {
    return new CardinalOpen(context, tension);
  }
  cardinal.tension = function(tension2) {
    return custom15(+tension2);
  };
  return cardinal;
}(0);
// node_modules/d3-shape/src/curve/catmullRom.js
function point4(that, x4, y4) {
  var { _x1: x12, _y1: y12, _x2: x22, _y2: y22 } = that;
  if (that._l01_a > epsilon9) {
    var a4 = 2 * that._l01_2a + 3 * that._l01_a * that._l12_a + that._l12_2a, n = 3 * that._l01_a * (that._l01_a + that._l12_a);
    x12 = (x12 * a4 - that._x0 * that._l12_2a + that._x2 * that._l01_2a) / n;
    y12 = (y12 * a4 - that._y0 * that._l12_2a + that._y2 * that._l01_2a) / n;
  }
  if (that._l23_a > epsilon9) {
    var b = 2 * that._l23_2a + 3 * that._l23_a * that._l12_a + that._l12_2a, m3 = 3 * that._l23_a * (that._l23_a + that._l12_a);
    x22 = (x22 * b + that._x1 * that._l23_2a - x4 * that._l12_2a) / m3;
    y22 = (y22 * b + that._y1 * that._l23_2a - y4 * that._l12_2a) / m3;
  }
  that._context.bezierCurveTo(x12, y12, x22, y22, that._x2, that._y2);
}
function CatmullRom(context, alpha) {
  this._context = context;
  this._alpha = alpha;
}
CatmullRom.prototype = {
  areaStart: function() {
    this._line = 0;
  },
  areaEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._x0 = this._x1 = this._x2 = this._y0 = this._y1 = this._y2 = NaN;
    this._l01_a = this._l12_a = this._l23_a = this._l01_2a = this._l12_2a = this._l23_2a = this._point = 0;
  },
  lineEnd: function() {
    switch (this._point) {
      case 2:
        this._context.lineTo(this._x2, this._y2);
        break;
      case 3:
        this.point(this._x2, this._y2);
        break;
    }
    if (this._line || this._line !== 0 && this._point === 1)
      this._context.closePath();
    this._line = 1 - this._line;
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    if (this._point) {
      var x23 = this._x2 - x4, y23 = this._y2 - y4;
      this._l23_a = Math.sqrt(this._l23_2a = Math.pow(x23 * x23 + y23 * y23, this._alpha));
    }
    switch (this._point) {
      case 0:
        this._point = 1;
        this._line ? this._context.lineTo(x4, y4) : this._context.moveTo(x4, y4);
        break;
      case 1:
        this._point = 2;
        break;
      case 2:
        this._point = 3;
      default:
        point4(this, x4, y4);
        break;
    }
    this._l01_a = this._l12_a, this._l12_a = this._l23_a;
    this._l01_2a = this._l12_2a, this._l12_2a = this._l23_2a;
    this._x0 = this._x1, this._x1 = this._x2, this._x2 = x4;
    this._y0 = this._y1, this._y1 = this._y2, this._y2 = y4;
  }
};
var catmullRom_default = function custom16(alpha) {
  function catmullRom(context) {
    return alpha ? new CatmullRom(context, alpha) : new Cardinal(context, 0);
  }
  catmullRom.alpha = function(alpha2) {
    return custom16(+alpha2);
  };
  return catmullRom;
}(0.5);

// node_modules/d3-shape/src/curve/catmullRomClosed.js
function CatmullRomClosed(context, alpha) {
  this._context = context;
  this._alpha = alpha;
}
CatmullRomClosed.prototype = {
  areaStart: noop_default2,
  areaEnd: noop_default2,
  lineStart: function() {
    this._x0 = this._x1 = this._x2 = this._x3 = this._x4 = this._x5 = this._y0 = this._y1 = this._y2 = this._y3 = this._y4 = this._y5 = NaN;
    this._l01_a = this._l12_a = this._l23_a = this._l01_2a = this._l12_2a = this._l23_2a = this._point = 0;
  },
  lineEnd: function() {
    switch (this._point) {
      case 1: {
        this._context.moveTo(this._x3, this._y3);
        this._context.closePath();
        break;
      }
      case 2: {
        this._context.lineTo(this._x3, this._y3);
        this._context.closePath();
        break;
      }
      case 3: {
        this.point(this._x3, this._y3);
        this.point(this._x4, this._y4);
        this.point(this._x5, this._y5);
        break;
      }
    }
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    if (this._point) {
      var x23 = this._x2 - x4, y23 = this._y2 - y4;
      this._l23_a = Math.sqrt(this._l23_2a = Math.pow(x23 * x23 + y23 * y23, this._alpha));
    }
    switch (this._point) {
      case 0:
        this._point = 1;
        this._x3 = x4, this._y3 = y4;
        break;
      case 1:
        this._point = 2;
        this._context.moveTo(this._x4 = x4, this._y4 = y4);
        break;
      case 2:
        this._point = 3;
        this._x5 = x4, this._y5 = y4;
        break;
      default:
        point4(this, x4, y4);
        break;
    }
    this._l01_a = this._l12_a, this._l12_a = this._l23_a;
    this._l01_2a = this._l12_2a, this._l12_2a = this._l23_2a;
    this._x0 = this._x1, this._x1 = this._x2, this._x2 = x4;
    this._y0 = this._y1, this._y1 = this._y2, this._y2 = y4;
  }
};
var catmullRomClosed_default = function custom17(alpha) {
  function catmullRom(context) {
    return alpha ? new CatmullRomClosed(context, alpha) : new CardinalClosed(context, 0);
  }
  catmullRom.alpha = function(alpha2) {
    return custom17(+alpha2);
  };
  return catmullRom;
}(0.5);
// node_modules/d3-shape/src/curve/catmullRomOpen.js
function CatmullRomOpen(context, alpha) {
  this._context = context;
  this._alpha = alpha;
}
CatmullRomOpen.prototype = {
  areaStart: function() {
    this._line = 0;
  },
  areaEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._x0 = this._x1 = this._x2 = this._y0 = this._y1 = this._y2 = NaN;
    this._l01_a = this._l12_a = this._l23_a = this._l01_2a = this._l12_2a = this._l23_2a = this._point = 0;
  },
  lineEnd: function() {
    if (this._line || this._line !== 0 && this._point === 3)
      this._context.closePath();
    this._line = 1 - this._line;
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    if (this._point) {
      var x23 = this._x2 - x4, y23 = this._y2 - y4;
      this._l23_a = Math.sqrt(this._l23_2a = Math.pow(x23 * x23 + y23 * y23, this._alpha));
    }
    switch (this._point) {
      case 0:
        this._point = 1;
        break;
      case 1:
        this._point = 2;
        break;
      case 2:
        this._point = 3;
        this._line ? this._context.lineTo(this._x2, this._y2) : this._context.moveTo(this._x2, this._y2);
        break;
      case 3:
        this._point = 4;
      default:
        point4(this, x4, y4);
        break;
    }
    this._l01_a = this._l12_a, this._l12_a = this._l23_a;
    this._l01_2a = this._l12_2a, this._l12_2a = this._l23_2a;
    this._x0 = this._x1, this._x1 = this._x2, this._x2 = x4;
    this._y0 = this._y1, this._y1 = this._y2, this._y2 = y4;
  }
};
var catmullRomOpen_default = function custom18(alpha) {
  function catmullRom(context) {
    return alpha ? new CatmullRomOpen(context, alpha) : new CardinalOpen(context, 0);
  }
  catmullRom.alpha = function(alpha2) {
    return custom18(+alpha2);
  };
  return catmullRom;
}(0.5);
// node_modules/d3-shape/src/curve/linearClosed.js
function LinearClosed(context) {
  this._context = context;
}
LinearClosed.prototype = {
  areaStart: noop_default2,
  areaEnd: noop_default2,
  lineStart: function() {
    this._point = 0;
  },
  lineEnd: function() {
    if (this._point)
      this._context.closePath();
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    if (this._point)
      this._context.lineTo(x4, y4);
    else
      this._point = 1, this._context.moveTo(x4, y4);
  }
};
function linearClosed_default(context) {
  return new LinearClosed(context);
}
// node_modules/d3-shape/src/curve/monotone.js
function sign2(x4) {
  return x4 < 0 ? -1 : 1;
}
function slope3(that, x22, y22) {
  var h0 = that._x1 - that._x0, h1 = x22 - that._x1, s0 = (that._y1 - that._y0) / (h0 || h1 < 0 && -0), s1 = (y22 - that._y1) / (h1 || h0 < 0 && -0), p = (s0 * h1 + s1 * h0) / (h0 + h1);
  return (sign2(s0) + sign2(s1)) * Math.min(Math.abs(s0), Math.abs(s1), 0.5 * Math.abs(p)) || 0;
}
function slope2(that, t) {
  var h = that._x1 - that._x0;
  return h ? (3 * (that._y1 - that._y0) / h - t) / 2 : t;
}
function point5(that, t03, t13) {
  var { _x0: x06, _y0: y06, _x1: x12, _y1: y12 } = that, dx = (x12 - x06) / 3;
  that._context.bezierCurveTo(x06 + dx, y06 + dx * t03, x12 - dx, y12 - dx * t13, x12, y12);
}
function MonotoneX(context) {
  this._context = context;
}
MonotoneX.prototype = {
  areaStart: function() {
    this._line = 0;
  },
  areaEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._x0 = this._x1 = this._y0 = this._y1 = this._t0 = NaN;
    this._point = 0;
  },
  lineEnd: function() {
    switch (this._point) {
      case 2:
        this._context.lineTo(this._x1, this._y1);
        break;
      case 3:
        point5(this, this._t0, slope2(this, this._t0));
        break;
    }
    if (this._line || this._line !== 0 && this._point === 1)
      this._context.closePath();
    this._line = 1 - this._line;
  },
  point: function(x4, y4) {
    var t13 = NaN;
    x4 = +x4, y4 = +y4;
    if (x4 === this._x1 && y4 === this._y1)
      return;
    switch (this._point) {
      case 0:
        this._point = 1;
        this._line ? this._context.lineTo(x4, y4) : this._context.moveTo(x4, y4);
        break;
      case 1:
        this._point = 2;
        break;
      case 2:
        this._point = 3;
        point5(this, slope2(this, t13 = slope3(this, x4, y4)), t13);
        break;
      default:
        point5(this, this._t0, t13 = slope3(this, x4, y4));
        break;
    }
    this._x0 = this._x1, this._x1 = x4;
    this._y0 = this._y1, this._y1 = y4;
    this._t0 = t13;
  }
};
function MonotoneY(context) {
  this._context = new ReflectContext(context);
}
(MonotoneY.prototype = Object.create(MonotoneX.prototype)).point = function(x4, y4) {
  MonotoneX.prototype.point.call(this, y4, x4);
};
function ReflectContext(context) {
  this._context = context;
}
ReflectContext.prototype = {
  moveTo: function(x4, y4) {
    this._context.moveTo(y4, x4);
  },
  closePath: function() {
    this._context.closePath();
  },
  lineTo: function(x4, y4) {
    this._context.lineTo(y4, x4);
  },
  bezierCurveTo: function(x12, y12, x22, y22, x4, y4) {
    this._context.bezierCurveTo(y12, x12, y22, x22, y4, x4);
  }
};
function monotoneX(context) {
  return new MonotoneX(context);
}
function monotoneY(context) {
  return new MonotoneY(context);
}
// node_modules/d3-shape/src/curve/natural.js
function Natural(context) {
  this._context = context;
}
Natural.prototype = {
  areaStart: function() {
    this._line = 0;
  },
  areaEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._x = [];
    this._y = [];
  },
  lineEnd: function() {
    var x4 = this._x, y4 = this._y, n = x4.length;
    if (n) {
      this._line ? this._context.lineTo(x4[0], y4[0]) : this._context.moveTo(x4[0], y4[0]);
      if (n === 2) {
        this._context.lineTo(x4[1], y4[1]);
      } else {
        var px = controlPoints(x4), py = controlPoints(y4);
        for (var i0 = 0, i1 = 1;i1 < n; ++i0, ++i1) {
          this._context.bezierCurveTo(px[0][i0], py[0][i0], px[1][i0], py[1][i0], x4[i1], y4[i1]);
        }
      }
    }
    if (this._line || this._line !== 0 && n === 1)
      this._context.closePath();
    this._line = 1 - this._line;
    this._x = this._y = null;
  },
  point: function(x4, y4) {
    this._x.push(+x4);
    this._y.push(+y4);
  }
};
function controlPoints(x4) {
  var i, n = x4.length - 1, m3, a4 = new Array(n), b = new Array(n), r = new Array(n);
  a4[0] = 0, b[0] = 2, r[0] = x4[0] + 2 * x4[1];
  for (i = 1;i < n - 1; ++i)
    a4[i] = 1, b[i] = 4, r[i] = 4 * x4[i] + 2 * x4[i + 1];
  a4[n - 1] = 2, b[n - 1] = 7, r[n - 1] = 8 * x4[n - 1] + x4[n];
  for (i = 1;i < n; ++i)
    m3 = a4[i] / b[i - 1], b[i] -= m3, r[i] -= m3 * r[i - 1];
  a4[n - 1] = r[n - 1] / b[n - 1];
  for (i = n - 2;i >= 0; --i)
    a4[i] = (r[i] - a4[i + 1]) / b[i];
  b[n - 1] = (x4[n] + a4[n - 1]) / 2;
  for (i = 0;i < n - 1; ++i)
    b[i] = 2 * x4[i + 1] - a4[i + 1];
  return [a4, b];
}
function natural_default(context) {
  return new Natural(context);
}
// node_modules/d3-shape/src/curve/step.js
function Step(context, t) {
  this._context = context;
  this._t = t;
}
Step.prototype = {
  areaStart: function() {
    this._line = 0;
  },
  areaEnd: function() {
    this._line = NaN;
  },
  lineStart: function() {
    this._x = this._y = NaN;
    this._point = 0;
  },
  lineEnd: function() {
    if (0 < this._t && this._t < 1 && this._point === 2)
      this._context.lineTo(this._x, this._y);
    if (this._line || this._line !== 0 && this._point === 1)
      this._context.closePath();
    if (this._line >= 0)
      this._t = 1 - this._t, this._line = 1 - this._line;
  },
  point: function(x4, y4) {
    x4 = +x4, y4 = +y4;
    switch (this._point) {
      case 0:
        this._point = 1;
        this._line ? this._context.lineTo(x4, y4) : this._context.moveTo(x4, y4);
        break;
      case 1:
        this._point = 2;
      default: {
        if (this._t <= 0) {
          this._context.lineTo(this._x, y4);
          this._context.lineTo(x4, y4);
        } else {
          var x12 = this._x * (1 - this._t) + x4 * this._t;
          this._context.lineTo(x12, this._y);
          this._context.lineTo(x12, y4);
        }
        break;
      }
    }
    this._x = x4, this._y = y4;
  }
};
function step_default(context) {
  return new Step(context, 0.5);
}
function stepBefore(context) {
  return new Step(context, 0);
}
function stepAfter(context) {
  return new Step(context, 1);
}
// node_modules/d3-shape/src/offset/none.js
function none_default(series, order) {
  if (!((n = series.length) > 1))
    return;
  for (var i = 1, j, s0, s1 = series[order[0]], n, m3 = s1.length;i < n; ++i) {
    s0 = s1, s1 = series[order[i]];
    for (j = 0;j < m3; ++j) {
      s1[j][1] += s1[j][0] = isNaN(s0[j][1]) ? s0[j][0] : s0[j][1];
    }
  }
}

// node_modules/d3-shape/src/order/none.js
function none_default2(series) {
  var n = series.length, o = new Array(n);
  while (--n >= 0)
    o[n] = n;
  return o;
}

// node_modules/d3-shape/src/stack.js
function stackValue(d, key) {
  return d[key];
}
function stackSeries(key) {
  const series = [];
  series.key = key;
  return series;
}
function stack_default() {
  var keys = constant_default10([]), order = none_default2, offset = none_default, value = stackValue;
  function stack(data) {
    var sz = Array.from(keys.apply(this, arguments), stackSeries), i, n = sz.length, j = -1, oz;
    for (const d of data) {
      for (i = 0, ++j;i < n; ++i) {
        (sz[i][j] = [0, +value(d, sz[i].key, j, data)]).data = d;
      }
    }
    for (i = 0, oz = array_default3(order(sz));i < n; ++i) {
      sz[oz[i]].index = i;
    }
    offset(sz, oz);
    return sz;
  }
  stack.keys = function(_) {
    return arguments.length ? (keys = typeof _ === "function" ? _ : constant_default10(Array.from(_)), stack) : keys;
  };
  stack.value = function(_) {
    return arguments.length ? (value = typeof _ === "function" ? _ : constant_default10(+_), stack) : value;
  };
  stack.order = function(_) {
    return arguments.length ? (order = _ == null ? none_default2 : typeof _ === "function" ? _ : constant_default10(Array.from(_)), stack) : order;
  };
  stack.offset = function(_) {
    return arguments.length ? (offset = _ == null ? none_default : _, stack) : offset;
  };
  return stack;
}
// node_modules/d3-shape/src/offset/expand.js
function expand_default(series, order) {
  if (!((n = series.length) > 0))
    return;
  for (var i, n, j = 0, m3 = series[0].length, y4;j < m3; ++j) {
    for (y4 = i = 0;i < n; ++i)
      y4 += series[i][j][1] || 0;
    if (y4)
      for (i = 0;i < n; ++i)
        series[i][j][1] /= y4;
  }
  none_default(series, order);
}
// node_modules/d3-shape/src/offset/diverging.js
function diverging_default(series, order) {
  if (!((n = series.length) > 0))
    return;
  for (var i, j = 0, d, dy, yp, yn, n, m3 = series[order[0]].length;j < m3; ++j) {
    for (yp = yn = 0, i = 0;i < n; ++i) {
      if ((dy = (d = series[order[i]][j])[1] - d[0]) > 0) {
        d[0] = yp, d[1] = yp += dy;
      } else if (dy < 0) {
        d[1] = yn, d[0] = yn += dy;
      } else {
        d[0] = 0, d[1] = dy;
      }
    }
  }
}
// node_modules/d3-shape/src/offset/silhouette.js
function silhouette_default(series, order) {
  if (!((n = series.length) > 0))
    return;
  for (var j = 0, s0 = series[order[0]], n, m3 = s0.length;j < m3; ++j) {
    for (var i = 0, y4 = 0;i < n; ++i)
      y4 += series[i][j][1] || 0;
    s0[j][1] += s0[j][0] = -y4 / 2;
  }
  none_default(series, order);
}
// node_modules/d3-shape/src/offset/wiggle.js
function wiggle_default(series, order) {
  if (!((n = series.length) > 0) || !((m3 = (s0 = series[order[0]]).length) > 0))
    return;
  for (var y4 = 0, j = 1, s0, m3, n;j < m3; ++j) {
    for (var i = 0, s1 = 0, s2 = 0;i < n; ++i) {
      var si = series[order[i]], sij0 = si[j][1] || 0, sij1 = si[j - 1][1] || 0, s3 = (sij0 - sij1) / 2;
      for (var k2 = 0;k2 < i; ++k2) {
        var sk = series[order[k2]], skj0 = sk[j][1] || 0, skj1 = sk[j - 1][1] || 0;
        s3 += skj0 - skj1;
      }
      s1 += sij0, s2 += s3 * sij0;
    }
    s0[j - 1][1] += s0[j - 1][0] = y4;
    if (s1)
      y4 -= s2 / s1;
  }
  s0[j - 1][1] += s0[j - 1][0] = y4;
  none_default(series, order);
}
// node_modules/d3-shape/src/order/appearance.js
function appearance_default(series) {
  var peaks = series.map(peak);
  return none_default2(series).sort(function(a4, b) {
    return peaks[a4] - peaks[b];
  });
}
function peak(series) {
  var i = -1, j = 0, n = series.length, vi, vj = -Infinity;
  while (++i < n)
    if ((vi = +series[i][1]) > vj)
      vj = vi, j = i;
  return j;
}
// node_modules/d3-shape/src/order/ascending.js
function ascending_default2(series) {
  var sums = series.map(sum3);
  return none_default2(series).sort(function(a4, b) {
    return sums[a4] - sums[b];
  });
}
function sum3(series) {
  var s2 = 0, i = -1, n = series.length, v2;
  while (++i < n)
    if (v2 = +series[i][1])
      s2 += v2;
  return s2;
}
// node_modules/d3-shape/src/order/descending.js
function descending_default2(series) {
  return ascending_default2(series).reverse();
}
// node_modules/d3-shape/src/order/insideOut.js
function insideOut_default(series) {
  var n = series.length, i, j, sums = series.map(sum3), order = appearance_default(series), top2 = 0, bottom2 = 0, tops = [], bottoms = [];
  for (i = 0;i < n; ++i) {
    j = order[i];
    if (top2 < bottom2) {
      top2 += sums[j];
      tops.push(j);
    } else {
      bottom2 += sums[j];
      bottoms.push(j);
    }
  }
  return bottoms.reverse().concat(tops);
}
// node_modules/d3-shape/src/order/reverse.js
function reverse_default(series) {
  return none_default2(series).reverse();
}
// node_modules/d3-zoom/src/constant.js
var constant_default11 = (x4) => () => x4;

// node_modules/d3-zoom/src/event.js
function ZoomEvent(type2, {
  sourceEvent,
  target,
  transform,
  dispatch: dispatch2
}) {
  Object.defineProperties(this, {
    type: { value: type2, enumerable: true, configurable: true },
    sourceEvent: { value: sourceEvent, enumerable: true, configurable: true },
    target: { value: target, enumerable: true, configurable: true },
    transform: { value: transform, enumerable: true, configurable: true },
    _: { value: dispatch2 }
  });
}

// node_modules/d3-zoom/src/transform.js
function Transform(k2, x4, y4) {
  this.k = k2;
  this.x = x4;
  this.y = y4;
}
Transform.prototype = {
  constructor: Transform,
  scale: function(k2) {
    return k2 === 1 ? this : new Transform(this.k * k2, this.x, this.y);
  },
  translate: function(x4, y4) {
    return x4 === 0 & y4 === 0 ? this : new Transform(this.k, this.x + this.k * x4, this.y + this.k * y4);
  },
  apply: function(point6) {
    return [point6[0] * this.k + this.x, point6[1] * this.k + this.y];
  },
  applyX: function(x4) {
    return x4 * this.k + this.x;
  },
  applyY: function(y4) {
    return y4 * this.k + this.y;
  },
  invert: function(location) {
    return [(location[0] - this.x) / this.k, (location[1] - this.y) / this.k];
  },
  invertX: function(x4) {
    return (x4 - this.x) / this.k;
  },
  invertY: function(y4) {
    return (y4 - this.y) / this.k;
  },
  rescaleX: function(x4) {
    return x4.copy().domain(x4.range().map(this.invertX, this).map(x4.invert, x4));
  },
  rescaleY: function(y4) {
    return y4.copy().domain(y4.range().map(this.invertY, this).map(y4.invert, y4));
  },
  toString: function() {
    return "translate(" + this.x + "," + this.y + ") scale(" + this.k + ")";
  }
};
var identity5 = new Transform(1, 0, 0);
transform.prototype = Transform.prototype;
function transform(node) {
  while (!node.__zoom)
    if (!(node = node.parentNode))
      return identity5;
  return node.__zoom;
}

// node_modules/d3-zoom/src/noevent.js
function nopropagation3(event) {
  event.stopImmediatePropagation();
}
function noevent_default3(event) {
  event.preventDefault();
  event.stopImmediatePropagation();
}

// node_modules/d3-zoom/src/zoom.js
function defaultFilter3(event) {
  return (!event.ctrlKey || event.type === "wheel") && !event.button;
}
function defaultExtent2() {
  var e = this;
  if (e instanceof SVGElement) {
    e = e.ownerSVGElement || e;
    if (e.hasAttribute("viewBox")) {
      e = e.viewBox.baseVal;
      return [[e.x, e.y], [e.x + e.width, e.y + e.height]];
    }
    return [[0, 0], [e.width.baseVal.value, e.height.baseVal.value]];
  }
  return [[0, 0], [e.clientWidth, e.clientHeight]];
}
function defaultTransform() {
  return this.__zoom || identity5;
}
function defaultWheelDelta(event) {
  return -event.deltaY * (event.deltaMode === 1 ? 0.05 : event.deltaMode ? 1 : 0.002) * (event.ctrlKey ? 10 : 1);
}
function defaultTouchable3() {
  return navigator.maxTouchPoints || "ontouchstart" in this;
}
function defaultConstrain(transform2, extent2, translateExtent) {
  var dx0 = transform2.invertX(extent2[0][0]) - translateExtent[0][0], dx1 = transform2.invertX(extent2[1][0]) - translateExtent[1][0], dy0 = transform2.invertY(extent2[0][1]) - translateExtent[0][1], dy1 = transform2.invertY(extent2[1][1]) - translateExtent[1][1];
  return transform2.translate(dx1 > dx0 ? (dx0 + dx1) / 2 : Math.min(0, dx0) || Math.max(0, dx1), dy1 > dy0 ? (dy0 + dy1) / 2 : Math.min(0, dy0) || Math.max(0, dy1));
}
function zoom_default2() {
  var filter3 = defaultFilter3, extent2 = defaultExtent2, constrain = defaultConstrain, wheelDelta = defaultWheelDelta, touchable = defaultTouchable3, scaleExtent = [0, Infinity], translateExtent = [[-Infinity, -Infinity], [Infinity, Infinity]], duration = 250, interpolate = zoom_default, listeners = dispatch_default("start", "zoom", "end"), touchstarting, touchfirst, touchending, touchDelay = 500, wheelDelay = 150, clickDistance2 = 0, tapDistance = 10;
  function zoom(selection2) {
    selection2.property("__zoom", defaultTransform).on("wheel.zoom", wheeled, { passive: false }).on("mousedown.zoom", mousedowned).on("dblclick.zoom", dblclicked).filter(touchable).on("touchstart.zoom", touchstarted).on("touchmove.zoom", touchmoved).on("touchend.zoom touchcancel.zoom", touchended).style("-webkit-tap-highlight-color", "rgba(0,0,0,0)");
  }
  zoom.transform = function(collection, transform2, point6, event) {
    var selection2 = collection.selection ? collection.selection() : collection;
    selection2.property("__zoom", defaultTransform);
    if (collection !== selection2) {
      schedule(collection, transform2, point6, event);
    } else {
      selection2.interrupt().each(function() {
        gesture(this, arguments).event(event).start().zoom(null, typeof transform2 === "function" ? transform2.apply(this, arguments) : transform2).end();
      });
    }
  };
  zoom.scaleBy = function(selection2, k2, p, event) {
    zoom.scaleTo(selection2, function() {
      var k0 = this.__zoom.k, k1 = typeof k2 === "function" ? k2.apply(this, arguments) : k2;
      return k0 * k1;
    }, p, event);
  };
  zoom.scaleTo = function(selection2, k2, p, event) {
    zoom.transform(selection2, function() {
      var e = extent2.apply(this, arguments), t03 = this.__zoom, p02 = p == null ? centroid(e) : typeof p === "function" ? p.apply(this, arguments) : p, p1 = t03.invert(p02), k1 = typeof k2 === "function" ? k2.apply(this, arguments) : k2;
      return constrain(translate(scale2(t03, k1), p02, p1), e, translateExtent);
    }, p, event);
  };
  zoom.translateBy = function(selection2, x4, y4, event) {
    zoom.transform(selection2, function() {
      return constrain(this.__zoom.translate(typeof x4 === "function" ? x4.apply(this, arguments) : x4, typeof y4 === "function" ? y4.apply(this, arguments) : y4), extent2.apply(this, arguments), translateExtent);
    }, null, event);
  };
  zoom.translateTo = function(selection2, x4, y4, p, event) {
    zoom.transform(selection2, function() {
      var e = extent2.apply(this, arguments), t = this.__zoom, p02 = p == null ? centroid(e) : typeof p === "function" ? p.apply(this, arguments) : p;
      return constrain(identity5.translate(p02[0], p02[1]).scale(t.k).translate(typeof x4 === "function" ? -x4.apply(this, arguments) : -x4, typeof y4 === "function" ? -y4.apply(this, arguments) : -y4), e, translateExtent);
    }, p, event);
  };
  function scale2(transform2, k2) {
    k2 = Math.max(scaleExtent[0], Math.min(scaleExtent[1], k2));
    return k2 === transform2.k ? transform2 : new Transform(k2, transform2.x, transform2.y);
  }
  function translate(transform2, p02, p1) {
    var x4 = p02[0] - p1[0] * transform2.k, y4 = p02[1] - p1[1] * transform2.k;
    return x4 === transform2.x && y4 === transform2.y ? transform2 : new Transform(transform2.k, x4, y4);
  }
  function centroid(extent3) {
    return [(+extent3[0][0] + +extent3[1][0]) / 2, (+extent3[0][1] + +extent3[1][1]) / 2];
  }
  function schedule(transition2, transform2, point6, event) {
    transition2.on("start.zoom", function() {
      gesture(this, arguments).event(event).start();
    }).on("interrupt.zoom end.zoom", function() {
      gesture(this, arguments).event(event).end();
    }).tween("zoom", function() {
      var that = this, args = arguments, g = gesture(that, args).event(event), e = extent2.apply(that, args), p = point6 == null ? centroid(e) : typeof point6 === "function" ? point6.apply(that, args) : point6, w = Math.max(e[1][0] - e[0][0], e[1][1] - e[0][1]), a4 = that.__zoom, b = typeof transform2 === "function" ? transform2.apply(that, args) : transform2, i = interpolate(a4.invert(p).concat(w / a4.k), b.invert(p).concat(w / b.k));
      return function(t) {
        if (t === 1)
          t = b;
        else {
          var l = i(t), k2 = w / l[2];
          t = new Transform(k2, p[0] - l[0] * k2, p[1] - l[1] * k2);
        }
        g.zoom(null, t);
      };
    });
  }
  function gesture(that, args, clean) {
    return !clean && that.__zooming || new Gesture(that, args);
  }
  function Gesture(that, args) {
    this.that = that;
    this.args = args;
    this.active = 0;
    this.sourceEvent = null;
    this.extent = extent2.apply(that, args);
    this.taps = 0;
  }
  Gesture.prototype = {
    event: function(event) {
      if (event)
        this.sourceEvent = event;
      return this;
    },
    start: function() {
      if (++this.active === 1) {
        this.that.__zooming = this;
        this.emit("start");
      }
      return this;
    },
    zoom: function(key, transform2) {
      if (this.mouse && key !== "mouse")
        this.mouse[1] = transform2.invert(this.mouse[0]);
      if (this.touch0 && key !== "touch")
        this.touch0[1] = transform2.invert(this.touch0[0]);
      if (this.touch1 && key !== "touch")
        this.touch1[1] = transform2.invert(this.touch1[0]);
      this.that.__zoom = transform2;
      this.emit("zoom");
      return this;
    },
    end: function() {
      if (--this.active === 0) {
        delete this.that.__zooming;
        this.emit("end");
      }
      return this;
    },
    emit: function(type2) {
      var d = select_default2(this.that).datum();
      listeners.call(type2, this.that, new ZoomEvent(type2, {
        sourceEvent: this.sourceEvent,
        target: zoom,
        type: type2,
        transform: this.that.__zoom,
        dispatch: listeners
      }), d);
    }
  };
  function wheeled(event, ...args) {
    if (!filter3.apply(this, arguments))
      return;
    var g = gesture(this, args).event(event), t = this.__zoom, k2 = Math.max(scaleExtent[0], Math.min(scaleExtent[1], t.k * Math.pow(2, wheelDelta.apply(this, arguments)))), p = pointer_default(event);
    if (g.wheel) {
      if (g.mouse[0][0] !== p[0] || g.mouse[0][1] !== p[1]) {
        g.mouse[1] = t.invert(g.mouse[0] = p);
      }
      clearTimeout(g.wheel);
    } else if (t.k === k2)
      return;
    else {
      g.mouse = [p, t.invert(p)];
      interrupt_default(this);
      g.start();
    }
    noevent_default3(event);
    g.wheel = setTimeout(wheelidled, wheelDelay);
    g.zoom("mouse", constrain(translate(scale2(t, k2), g.mouse[0], g.mouse[1]), g.extent, translateExtent));
    function wheelidled() {
      g.wheel = null;
      g.end();
    }
  }
  function mousedowned(event, ...args) {
    if (touchending || !filter3.apply(this, arguments))
      return;
    var currentTarget = event.currentTarget, g = gesture(this, args, true).event(event), v2 = select_default2(event.view).on("mousemove.zoom", mousemoved, true).on("mouseup.zoom", mouseupped, true), p = pointer_default(event, currentTarget), x06 = event.clientX, y06 = event.clientY;
    nodrag_default(event.view);
    nopropagation3(event);
    g.mouse = [p, this.__zoom.invert(p)];
    interrupt_default(this);
    g.start();
    function mousemoved(event2) {
      noevent_default3(event2);
      if (!g.moved) {
        var dx = event2.clientX - x06, dy = event2.clientY - y06;
        g.moved = dx * dx + dy * dy > clickDistance2;
      }
      g.event(event2).zoom("mouse", constrain(translate(g.that.__zoom, g.mouse[0] = pointer_default(event2, currentTarget), g.mouse[1]), g.extent, translateExtent));
    }
    function mouseupped(event2) {
      v2.on("mousemove.zoom mouseup.zoom", null);
      yesdrag(event2.view, g.moved);
      noevent_default3(event2);
      g.event(event2).end();
    }
  }
  function dblclicked(event, ...args) {
    if (!filter3.apply(this, arguments))
      return;
    var t03 = this.__zoom, p02 = pointer_default(event.changedTouches ? event.changedTouches[0] : event, this), p1 = t03.invert(p02), k1 = t03.k * (event.shiftKey ? 0.5 : 2), t13 = constrain(translate(scale2(t03, k1), p02, p1), extent2.apply(this, args), translateExtent);
    noevent_default3(event);
    if (duration > 0)
      select_default2(this).transition().duration(duration).call(schedule, t13, p02, event);
    else
      select_default2(this).call(zoom.transform, t13, p02, event);
  }
  function touchstarted(event, ...args) {
    if (!filter3.apply(this, arguments))
      return;
    var touches = event.touches, n = touches.length, g = gesture(this, args, event.changedTouches.length === n).event(event), started, i, t, p;
    nopropagation3(event);
    for (i = 0;i < n; ++i) {
      t = touches[i], p = pointer_default(t, this);
      p = [p, this.__zoom.invert(p), t.identifier];
      if (!g.touch0)
        g.touch0 = p, started = true, g.taps = 1 + !!touchstarting;
      else if (!g.touch1 && g.touch0[2] !== p[2])
        g.touch1 = p, g.taps = 0;
    }
    if (touchstarting)
      touchstarting = clearTimeout(touchstarting);
    if (started) {
      if (g.taps < 2)
        touchfirst = p[0], touchstarting = setTimeout(function() {
          touchstarting = null;
        }, touchDelay);
      interrupt_default(this);
      g.start();
    }
  }
  function touchmoved(event, ...args) {
    if (!this.__zooming)
      return;
    var g = gesture(this, args).event(event), touches = event.changedTouches, n = touches.length, i, t, p, l;
    noevent_default3(event);
    for (i = 0;i < n; ++i) {
      t = touches[i], p = pointer_default(t, this);
      if (g.touch0 && g.touch0[2] === t.identifier)
        g.touch0[0] = p;
      else if (g.touch1 && g.touch1[2] === t.identifier)
        g.touch1[0] = p;
    }
    t = g.that.__zoom;
    if (g.touch1) {
      var p02 = g.touch0[0], l0 = g.touch0[1], p1 = g.touch1[0], l1 = g.touch1[1], dp = (dp = p1[0] - p02[0]) * dp + (dp = p1[1] - p02[1]) * dp, dl = (dl = l1[0] - l0[0]) * dl + (dl = l1[1] - l0[1]) * dl;
      t = scale2(t, Math.sqrt(dp / dl));
      p = [(p02[0] + p1[0]) / 2, (p02[1] + p1[1]) / 2];
      l = [(l0[0] + l1[0]) / 2, (l0[1] + l1[1]) / 2];
    } else if (g.touch0)
      p = g.touch0[0], l = g.touch0[1];
    else
      return;
    g.zoom("touch", constrain(translate(t, p, l), g.extent, translateExtent));
  }
  function touchended(event, ...args) {
    if (!this.__zooming)
      return;
    var g = gesture(this, args).event(event), touches = event.changedTouches, n = touches.length, i, t;
    nopropagation3(event);
    if (touchending)
      clearTimeout(touchending);
    touchending = setTimeout(function() {
      touchending = null;
    }, touchDelay);
    for (i = 0;i < n; ++i) {
      t = touches[i];
      if (g.touch0 && g.touch0[2] === t.identifier)
        delete g.touch0;
      else if (g.touch1 && g.touch1[2] === t.identifier)
        delete g.touch1;
    }
    if (g.touch1 && !g.touch0)
      g.touch0 = g.touch1, delete g.touch1;
    if (g.touch0)
      g.touch0[1] = this.__zoom.invert(g.touch0[0]);
    else {
      g.end();
      if (g.taps === 2) {
        t = pointer_default(t, this);
        if (Math.hypot(touchfirst[0] - t[0], touchfirst[1] - t[1]) < tapDistance) {
          var p = select_default2(this).on("dblclick.zoom");
          if (p)
            p.apply(this, arguments);
        }
      }
    }
  }
  zoom.wheelDelta = function(_) {
    return arguments.length ? (wheelDelta = typeof _ === "function" ? _ : constant_default11(+_), zoom) : wheelDelta;
  };
  zoom.filter = function(_) {
    return arguments.length ? (filter3 = typeof _ === "function" ? _ : constant_default11(!!_), zoom) : filter3;
  };
  zoom.touchable = function(_) {
    return arguments.length ? (touchable = typeof _ === "function" ? _ : constant_default11(!!_), zoom) : touchable;
  };
  zoom.extent = function(_) {
    return arguments.length ? (extent2 = typeof _ === "function" ? _ : constant_default11([[+_[0][0], +_[0][1]], [+_[1][0], +_[1][1]]]), zoom) : extent2;
  };
  zoom.scaleExtent = function(_) {
    return arguments.length ? (scaleExtent[0] = +_[0], scaleExtent[1] = +_[1], zoom) : [scaleExtent[0], scaleExtent[1]];
  };
  zoom.translateExtent = function(_) {
    return arguments.length ? (translateExtent[0][0] = +_[0][0], translateExtent[1][0] = +_[1][0], translateExtent[0][1] = +_[0][1], translateExtent[1][1] = +_[1][1], zoom) : [[translateExtent[0][0], translateExtent[0][1]], [translateExtent[1][0], translateExtent[1][1]]];
  };
  zoom.constrain = function(_) {
    return arguments.length ? (constrain = _, zoom) : constrain;
  };
  zoom.duration = function(_) {
    return arguments.length ? (duration = +_, zoom) : duration;
  };
  zoom.interpolate = function(_) {
    return arguments.length ? (interpolate = _, zoom) : interpolate;
  };
  zoom.on = function() {
    var value = listeners.on.apply(listeners, arguments);
    return value === listeners ? zoom : value;
  };
  zoom.clickDistance = function(_) {
    return arguments.length ? (clickDistance2 = (_ = +_) * _, zoom) : Math.sqrt(clickDistance2);
  };
  zoom.tapDistance = function(_) {
    return arguments.length ? (tapDistance = +_, zoom) : tapDistance;
  };
  return zoom;
}
export {
  transform as zoomTransform,
  identity5 as zoomIdentity,
  zoom_default2 as zoom,
  zip,
  xml_default as xml,
  window_default as window,
  variance,
  utcYears,
  utcYear,
  utcSundays as utcWeeks,
  utcSunday as utcWeek,
  utcWednesdays,
  utcWednesday,
  utcTuesdays,
  utcTuesday,
  utcTicks,
  utcTickInterval,
  utcThursdays,
  utcThursday,
  utcSundays,
  utcSunday,
  seconds as utcSeconds,
  second as utcSecond,
  utcSaturdays,
  utcSaturday,
  utcParse,
  utcMonths,
  utcMonth,
  utcMondays,
  utcMonday,
  utcMinutes,
  utcMinute,
  milliseconds as utcMilliseconds,
  millisecond as utcMillisecond,
  utcHours,
  utcHour,
  utcFridays,
  utcFriday,
  utcFormat,
  utcDays,
  utcDay,
  unixDays,
  unixDay,
  union,
  tsvParseRows,
  tsvParse,
  tsvFormatValue,
  tsvFormatRows,
  tsvFormatRow,
  tsvFormatBody,
  tsvFormat,
  tsv2 as tsv,
  squarify_default as treemapSquarify,
  sliceDice_default as treemapSliceDice,
  slice_default as treemapSlice,
  resquarify_default as treemapResquarify,
  dice_default as treemapDice,
  binary_default as treemapBinary,
  treemap_default as treemap,
  tree_default as tree,
  transpose,
  transition,
  timerFlush,
  timer,
  timeout_default as timeout,
  timeYears,
  timeYear,
  timeSundays as timeWeeks,
  timeSunday as timeWeek,
  timeWednesdays,
  timeWednesday,
  timeTuesdays,
  timeTuesday,
  timeTicks,
  timeTickInterval,
  timeThursdays,
  timeThursday,
  timeSundays,
  timeSunday,
  seconds as timeSeconds,
  second as timeSecond,
  timeSaturdays,
  timeSaturday,
  timeParse,
  timeMonths,
  timeMonth,
  timeMondays,
  timeMonday,
  timeMinutes,
  timeMinute,
  milliseconds as timeMilliseconds,
  millisecond as timeMillisecond,
  timeInterval,
  timeHours,
  timeHour,
  timeFridays,
  timeFriday,
  formatLocale as timeFormatLocale,
  defaultLocale2 as timeFormatDefaultLocale,
  timeFormat,
  timeDays,
  timeDay,
  ticks,
  tickStep,
  tickIncrement,
  tickFormat,
  thresholdSturges,
  thresholdScott,
  thresholdFreedmanDiaconis,
  text_default3 as text,
  symbolsStroke,
  symbolsFill,
  symbolsFill as symbols,
  times_default as symbolX,
  wye_default as symbolWye,
  triangle2_default as symbolTriangle2,
  triangle_default as symbolTriangle,
  times_default as symbolTimes,
  star_default as symbolStar,
  square2_default as symbolSquare2,
  square_default as symbolSquare,
  plus_default as symbolPlus,
  diamond2_default as symbolDiamond2,
  diamond_default as symbolDiamond,
  cross_default2 as symbolCross,
  circle_default3 as symbolCircle,
  asterisk_default as symbolAsterisk,
  Symbol2 as symbol,
  svg,
  superset,
  sum,
  subset,
  styleValue as style,
  stratify_default as stratify,
  reverse_default as stackOrderReverse,
  none_default2 as stackOrderNone,
  insideOut_default as stackOrderInsideOut,
  descending_default2 as stackOrderDescending,
  ascending_default2 as stackOrderAscending,
  appearance_default as stackOrderAppearance,
  wiggle_default as stackOffsetWiggle,
  silhouette_default as stackOffsetSilhouette,
  none_default as stackOffsetNone,
  expand_default as stackOffsetExpand,
  diverging_default as stackOffsetDiverging,
  stack_default as stack,
  sort,
  some,
  shuffler,
  shuffle_default as shuffle,
  selectorAll_default as selectorAll,
  selector_default as selector,
  selection_default as selection,
  selectAll_default2 as selectAll,
  select_default2 as select,
  scheme21 as schemeYlOrRd,
  scheme20 as schemeYlOrBr,
  scheme18 as schemeYlGnBu,
  scheme19 as schemeYlGn,
  Tableau10_default as schemeTableau10,
  scheme9 as schemeSpectral,
  Set3_default as schemeSet3,
  Set2_default as schemeSet2,
  Set1_default as schemeSet1,
  scheme26 as schemeReds,
  scheme8 as schemeRdYlGn,
  scheme7 as schemeRdYlBu,
  scheme17 as schemeRdPu,
  scheme6 as schemeRdGy,
  scheme5 as schemeRdBu,
  scheme25 as schemePurples,
  scheme16 as schemePuRd,
  scheme4 as schemePuOr,
  scheme14 as schemePuBuGn,
  scheme15 as schemePuBu,
  scheme3 as schemePiYG,
  Pastel2_default as schemePastel2,
  Pastel1_default as schemePastel1,
  Paired_default as schemePaired,
  scheme2 as schemePRGn,
  scheme27 as schemeOranges,
  scheme13 as schemeOrRd,
  observable10_default as schemeObservable10,
  scheme24 as schemeGreys,
  scheme23 as schemeGreens,
  scheme12 as schemeGnBu,
  Dark2_default as schemeDark2,
  category10_default as schemeCategory10,
  scheme11 as schemeBuPu,
  scheme10 as schemeBuGn,
  scheme as schemeBrBG,
  scheme22 as schemeBlues,
  Accent_default as schemeAccent,
  scan,
  utcTime as scaleUtc,
  time as scaleTime,
  threshold as scaleThreshold,
  symlog as scaleSymlog,
  sqrt2 as scaleSqrt,
  sequentialSymlog as scaleSequentialSymlog,
  sequentialSqrt as scaleSequentialSqrt,
  sequentialQuantile as scaleSequentialQuantile,
  sequentialPow as scaleSequentialPow,
  sequentialLog as scaleSequentialLog,
  sequential as scaleSequential,
  radial as scaleRadial,
  quantize as scaleQuantize,
  quantile2 as scaleQuantile,
  pow3 as scalePow,
  point as scalePoint,
  ordinal as scaleOrdinal,
  log2 as scaleLog,
  linear3 as scaleLinear,
  implicit as scaleImplicit,
  identity4 as scaleIdentity,
  divergingSymlog as scaleDivergingSymlog,
  divergingSqrt as scaleDivergingSqrt,
  divergingPow as scaleDivergingPow,
  divergingLog as scaleDivergingLog,
  diverging as scaleDiverging,
  band as scaleBand,
  rollups,
  rollup,
  ribbonArrow,
  ribbon_default as ribbon,
  rgb,
  reverse,
  reduce,
  rank,
  range,
  weibull_default as randomWeibull,
  uniform_default as randomUniform,
  poisson_default as randomPoisson,
  pareto_default as randomPareto,
  normal_default as randomNormal,
  logistic_default as randomLogistic,
  logNormal_default as randomLogNormal,
  lcg as randomLcg,
  irwinHall_default as randomIrwinHall,
  int_default as randomInt,
  geometric_default as randomGeometric,
  gamma_default as randomGamma,
  exponential_default as randomExponential,
  cauchy_default as randomCauchy,
  binomial_default as randomBinomial,
  beta_default as randomBeta,
  bernoulli_default as randomBernoulli,
  bates_default as randomBates,
  lineRadial_default as radialLine,
  areaRadial_default as radialArea,
  quickselect,
  quantize_default as quantize,
  quantileSorted,
  quantileIndex,
  quantile,
  quadtree,
  precisionRound_default as precisionRound,
  precisionPrefix_default as precisionPrefix,
  precisionFixed_default as precisionFixed,
  length_default2 as polygonLength,
  hull_default as polygonHull,
  contains_default3 as polygonContains,
  centroid_default3 as polygonCentroid,
  area_default4 as polygonArea,
  pointers_default as pointers,
  pointer_default as pointer,
  pointRadial_default as pointRadial,
  piecewise,
  pie_default as pie,
  permute,
  pathRound,
  path,
  partition_default as partition,
  pairs,
  siblings_default as packSiblings,
  enclose_default as packEnclose,
  pack_default as pack,
  now,
  nice,
  namespaces_default as namespaces,
  namespace_default as namespace,
  mode,
  minIndex,
  min,
  merge,
  medianIndex,
  median,
  mean,
  maxIndex,
  max,
  matcher_default as matcher,
  map2 as map,
  local,
  linkVertical,
  linkRadial,
  linkHorizontal,
  link2 as link,
  lineRadial_default as lineRadial,
  line_default2 as line,
  leastIndex,
  least,
  lch,
  lab,
  json_default as json,
  isoParse_default as isoParse,
  isoFormat_default as isoFormat,
  interval_default as interval,
  intersection,
  interrupt_default as interrupt,
  zoom_default as interpolateZoom,
  YlOrRd_default as interpolateYlOrRd,
  YlOrBr_default as interpolateYlOrBr,
  YlGnBu_default as interpolateYlGnBu,
  YlGn_default as interpolateYlGn,
  warm as interpolateWarm,
  viridis_default as interpolateViridis,
  turbo_default as interpolateTurbo,
  interpolateTransformSvg,
  interpolateTransformCss,
  string_default as interpolateString,
  Spectral_default as interpolateSpectral,
  sinebow_default as interpolateSinebow,
  round_default as interpolateRound,
  rgbBasisClosed as interpolateRgbBasisClosed,
  rgbBasis as interpolateRgbBasis,
  rgb_default as interpolateRgb,
  Reds_default as interpolateReds,
  RdYlGn_default as interpolateRdYlGn,
  RdYlBu_default as interpolateRdYlBu,
  RdPu_default as interpolateRdPu,
  RdGy_default as interpolateRdGy,
  RdBu_default as interpolateRdBu,
  rainbow_default as interpolateRainbow,
  Purples_default as interpolatePurples,
  PuRd_default as interpolatePuRd,
  PuOr_default as interpolatePuOr,
  PuBuGn_default as interpolatePuBuGn,
  PuBu_default as interpolatePuBu,
  plasma as interpolatePlasma,
  PiYG_default as interpolatePiYG,
  PRGn_default as interpolatePRGn,
  Oranges_default as interpolateOranges,
  OrRd_default as interpolateOrRd,
  object_default as interpolateObject,
  numberArray_default as interpolateNumberArray,
  number_default as interpolateNumber,
  magma as interpolateMagma,
  lab2 as interpolateLab,
  inferno as interpolateInferno,
  hue_default as interpolateHue,
  hslLong as interpolateHslLong,
  hsl_default as interpolateHsl,
  hclLong as interpolateHclLong,
  hcl_default as interpolateHcl,
  Greys_default as interpolateGreys,
  Greens_default as interpolateGreens,
  GnBu_default as interpolateGnBu,
  discrete_default as interpolateDiscrete,
  date_default as interpolateDate,
  cubehelixLong as interpolateCubehelixLong,
  cubehelix_default2 as interpolateCubehelixDefault,
  cubehelix_default as interpolateCubehelix,
  cool as interpolateCool,
  cividis_default as interpolateCividis,
  BuPu_default as interpolateBuPu,
  BuGn_default as interpolateBuGn,
  BrBG_default as interpolateBrBG,
  Blues_default as interpolateBlues,
  basisClosed_default as interpolateBasisClosed,
  basis_default as interpolateBasis,
  array_default as interpolateArray,
  value_default as interpolate,
  indexes,
  index,
  image_default as image,
  html,
  hsl,
  bin as histogram,
  hierarchy,
  hcl,
  groups,
  groupSort,
  group,
  greatestIndex,
  greatest,
  gray,
  transverseMercatorRaw as geoTransverseMercatorRaw,
  transverseMercator_default as geoTransverseMercator,
  transform_default as geoTransform,
  stream_default as geoStream,
  stereographicRaw as geoStereographicRaw,
  stereographic_default as geoStereographic,
  rotation_default as geoRotation,
  projectionMutator as geoProjectionMutator,
  projection as geoProjection,
  path_default as geoPath,
  orthographicRaw as geoOrthographicRaw,
  orthographic_default as geoOrthographic,
  naturalEarth1Raw as geoNaturalEarth1Raw,
  naturalEarth1_default as geoNaturalEarth1,
  mercatorRaw as geoMercatorRaw,
  mercator_default as geoMercator,
  length_default as geoLength,
  interpolate_default2 as geoInterpolate,
  identity_default4 as geoIdentity,
  graticule10 as geoGraticule10,
  graticule as geoGraticule,
  gnomonicRaw as geoGnomonicRaw,
  gnomonic_default as geoGnomonic,
  equirectangularRaw as geoEquirectangularRaw,
  equirectangular_default as geoEquirectangular,
  equalEarthRaw as geoEqualEarthRaw,
  equalEarth_default as geoEqualEarth,
  distance_default as geoDistance,
  contains_default2 as geoContains,
  conicEquidistantRaw as geoConicEquidistantRaw,
  conicEquidistant_default as geoConicEquidistant,
  conicEqualAreaRaw as geoConicEqualAreaRaw,
  conicEqualArea_default as geoConicEqualArea,
  conicConformalRaw as geoConicConformalRaw,
  conicConformal_default as geoConicConformal,
  clipRectangle as geoClipRectangle,
  extent_default2 as geoClipExtent,
  circle_default2 as geoClipCircle,
  antimeridian_default as geoClipAntimeridian,
  circle_default as geoCircle,
  centroid_default as geoCentroid,
  bounds_default as geoBounds,
  azimuthalEquidistantRaw as geoAzimuthalEquidistantRaw,
  azimuthalEquidistant_default as geoAzimuthalEquidistant,
  azimuthalEqualAreaRaw as geoAzimuthalEqualAreaRaw,
  azimuthalEqualArea_default as geoAzimuthalEqualArea,
  area_default2 as geoArea,
  albersUsa_default as geoAlbersUsa,
  albers_default as geoAlbers,
  fsum,
  formatSpecifier,
  formatPrefix,
  locale_default as formatLocale,
  defaultLocale as formatDefaultLocale,
  format,
  y_default2 as forceY,
  x_default2 as forceX,
  simulation_default as forceSimulation,
  radial_default as forceRadial,
  manyBody_default as forceManyBody,
  link_default as forceLink,
  collide_default as forceCollide,
  center_default as forceCenter,
  flatRollup,
  flatGroup,
  filter,
  fcumsum,
  extent,
  every,
  sinOut as easeSinOut,
  sinInOut as easeSinInOut,
  sinIn as easeSinIn,
  sinInOut as easeSin,
  quadOut as easeQuadOut,
  quadInOut as easeQuadInOut,
  quadIn as easeQuadIn,
  quadInOut as easeQuad,
  polyOut as easePolyOut,
  polyInOut as easePolyInOut,
  polyIn as easePolyIn,
  polyInOut as easePoly,
  linear2 as easeLinear,
  expOut as easeExpOut,
  expInOut as easeExpInOut,
  expIn as easeExpIn,
  expInOut as easeExp,
  elasticOut as easeElasticOut,
  elasticInOut as easeElasticInOut,
  elasticIn as easeElasticIn,
  elasticOut as easeElastic,
  cubicOut as easeCubicOut,
  cubicInOut as easeCubicInOut,
  cubicIn as easeCubicIn,
  cubicInOut as easeCubic,
  circleOut as easeCircleOut,
  circleInOut as easeCircleInOut,
  circleIn as easeCircleIn,
  circleInOut as easeCircle,
  bounceOut as easeBounceOut,
  bounceInOut as easeBounceInOut,
  bounceIn as easeBounceIn,
  bounceOut as easeBounce,
  backOut as easeBackOut,
  backInOut as easeBackInOut,
  backIn as easeBackIn,
  backInOut as easeBack,
  dsv_default as dsvFormat,
  dsv,
  yesdrag as dragEnable,
  nodrag_default as dragDisable,
  drag_default as drag,
  dispatch_default as dispatch,
  disjoint,
  difference,
  deviation,
  descending,
  stepBefore as curveStepBefore,
  stepAfter as curveStepAfter,
  step_default as curveStep,
  natural_default as curveNatural,
  monotoneY as curveMonotoneY,
  monotoneX as curveMonotoneX,
  linearClosed_default as curveLinearClosed,
  linear_default as curveLinear,
  catmullRomOpen_default as curveCatmullRomOpen,
  catmullRomClosed_default as curveCatmullRomClosed,
  catmullRom_default as curveCatmullRom,
  cardinalOpen_default as curveCardinalOpen,
  cardinalClosed_default as curveCardinalClosed,
  cardinal_default as curveCardinal,
  bundle_default as curveBundle,
  bumpY as curveBumpY,
  bumpX as curveBumpX,
  basisOpen_default as curveBasisOpen,
  basisClosed_default2 as curveBasisClosed,
  basis_default2 as curveBasis,
  cumsum,
  cubehelix,
  csvParseRows,
  csvParse,
  csvFormatValue,
  csvFormatRows,
  csvFormatRow,
  csvFormatBody,
  csvFormat,
  csv2 as csv,
  cross,
  creator_default as creator,
  create_default as create,
  count,
  contours_default as contours,
  density_default as contourDensity,
  color,
  cluster_default as cluster,
  chordTranspose,
  chordDirected,
  chord_default as chord,
  buffer_default as buffer,
  brushY,
  brushX,
  brushSelection,
  brush_default as brush,
  blurImage,
  blur2,
  blur,
  blob_default as blob,
  bisector,
  bisectRight,
  bisectLeft,
  bisectCenter,
  bisect_default as bisect,
  bin,
  axisTop,
  axisRight,
  axisLeft,
  axisBottom,
  autoType,
  ascending,
  areaRadial_default as areaRadial,
  area_default5 as area,
  arc_default as arc,
  active_default as active,
  Transform as ZoomTransform,
  Voronoi,
  Path,
  Node,
  InternSet,
  InternMap,
  FormatSpecifier,
  Delaunay,
  Adder
};
