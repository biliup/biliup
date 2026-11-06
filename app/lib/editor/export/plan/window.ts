import type {VisualLayer} from './types';
import {rampsAt} from './layers';

/** Sample in original timeline time first; shift only the finished render layer's clock. */
export function windowLayer(layer:VisualLayer,from:number):VisualLayer {
  const ramps=layer.ramps.map(ramp=>({...ramp,st:ramp.st-from}));
  // FFmpeg fade cannot begin at a negative timestamp. Bake a partial fade into the
  // existing alpha samples, preserving its original slope and level at the first frame.
  const partial=ramps.some(ramp=>ramp.st<0);
  const alpha=partial?Array.from({length:layer.f1-layer.f0},(_,index)=>(layer.alpha?.[index]??1)*rampsAt(layer.ramps,layer.f0+index)):layer.alpha;
  return {...layer,f0:layer.f0-from,f1:layer.f1-from,ramps:partial?[]:ramps,
    ...(alpha?{alpha}:{}),
    ...(layer.wipe?{wipe:{...layer.wipe,st:layer.wipe.st-from}}:{}),
    ...(layer.iris?{iris:{...layer.iris,st:layer.iris.st-from}}:{}),
    ...(layer.mask?{mask:{...layer.mask,st:layer.mask.st-from}}:{}),
    ...(layer.masks?{masks:{...layer.masks,start:layer.masks.start-from}}:{}),
    ...((layer.kind==='video'||layer.kind==='image')&&layer.fx?{fx:{...layer.fx,start:layer.fx.start-from}}:{}),
  };
}
