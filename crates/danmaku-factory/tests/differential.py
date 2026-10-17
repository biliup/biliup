#!/usr/bin/env python3
"""Fixed-seed external-process differential test of defined Legacy inputs.

Input is sorted to isolate layout from IC01/02. Bodies stay below old truncation
limits and contain no entity literals affected by IC05. Failing assets persist.
"""
import argparse
import json
from pathlib import Path
import random
import subprocess
import tempfile
from xml.sax.saxutils import escape


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--c',required=True,type=Path)
    parser.add_argument('--rust',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--cases',type=int,default=256)
    args=parser.parse_args()
    args.output.mkdir(parents=True,exist_ok=True)
    rng=random.Random(0x3813e93b)
    reports=[]
    for case in range(args.cases):
        options={
            'width':rng.choice([4,64,640,1280,1920]),'height':rng.choice([1,38,72,720,1080]),
            'display_area':rng.choice([0.0001,0.1,0.4,1.0]),'scroll_area':rng.choice([0.25,0.5,1.0]),
            'scroll_time':rng.choice([0.0001,0.123,3.33,12.0,3600.0]),'fix_time':rng.choice([0.01,1.0,5.5,12.0]),
            'density':rng.choice([-1,0,1,5,100]),'line_spacing':rng.choice([-20,0,1,20]),
            'top_margin':rng.choice([-10,0,20]),'bottom_margin':rng.choice([-10,0,20]),
            'font_size':rng.choice([25,38,72]),'font_size_strict':rng.choice([True,False]),'font_size_norm':rng.choice([True,False]),
            'opacity':rng.choice([1,180,255]),'outline_opacity':rng.choice([0,127,255]),
            'outline':rng.choice([0.0,1.25,4.0]),'outline_blur':rng.choice([0.0,0.00001,2.345678,100.0]),
            'shadow':rng.choice([0.0,0.75,4.0]),'bold':rng.choice([True,False]),
            'save_blocked':rng.choice([True,False]),'block_mode':rng.choice([0,1,2,4,8,16,32,64,127]),
        }
        elapsed=0
        records=[]
        for index in range(rng.randint(8,128)):
            elapsed+=rng.choice([0,0,1,4,10,100,1000,20000])
            mode=rng.choice([1,2,3,4,5,6,7,8,9,0,-1,-2,-3,-4])
            size=rng.choice([10,25,36,72,100])
            color=rng.choice([0xffffff,0,0x123456,0xff0000])
            body=rng.choice(['repeat','text '+str(index),'中文😀','a b {x} \\N','x'*rng.randint(1,80)])
            if mode==7:
                body=f'[{rng.choice([0,0.1,0.5,100])},{rng.choice([0,0.2,200])},"{rng.choice(["1-1","1-0.5","0.2-0.8"])}",{rng.choice([0,0.014,3,4.5])},"special/ntext",{rng.choice([0,30,-45])},{rng.choice([0,60])},{rng.choice([0,0.8,100])},{rng.choice([0,0.9,200])},{rng.choice([0,500,1500])},{rng.choice([0,200])},true,"font",1]'
            records.append(f'<d p="{elapsed//1000}.{elapsed%1000:03},{mode},{size},{color},0,0,0,0">{escape(body)}</d>')
        xml=('<i>\n'+'\n'.join(records)+'\n</i>').encode()
        with tempfile.TemporaryDirectory(prefix='df-differential-') as work:
            work=Path(work)
            input=work/'in.xml';input.write_bytes(xml)
            outputs=[]
            for name,binary in [('c',args.c),('rust',args.rust)]:
                output=work/f'{name}.ass'
                proc=subprocess.run([str(binary.resolve()),str(input),str(output),json.dumps(options)],capture_output=True,timeout=120)
                outputs.append((proc.returncode,output.read_bytes() if output.exists() else b'',proc.stdout,proc.stderr))
            if outputs[0]!=outputs[1]:
                # The display-only version differs; stats and result must not.
                equal=outputs[0][0:2]==outputs[1][0:2] and outputs[0][2].split(b' (')[0]==outputs[1][2].split(b' (')[0]
                if not equal:
                    artifact=args.output/f'case-{case}'
                    artifact.mkdir(exist_ok=True)
                    (artifact/'input.xml').write_bytes(xml)
                    (artifact/'options.json').write_text(json.dumps(options,indent=2))
                    for name,result in zip(['c','rust'],outputs):
                        (artifact/f'{name}.ass').write_bytes(result[1]);(artifact/f'{name}.stderr').write_bytes(result[3])
                    left,right=outputs[0][1],outputs[1][1]
                    at=next((at for at,(a,b) in enumerate(zip(left,right)) if a!=b),min(len(left),len(right)))
                    report={'case':case,'seed':'0x3813e93b','stage':'layout/writer/stats requires diagnosis','first_byte':at,'line':left[:at].count(b'\n')+1,'exit_codes':[r[0] for r in outputs]}
                    (artifact/'difference.json').write_text(json.dumps(report,indent=2))
                    raise SystemExit(json.dumps(report))
            reports.append({'case':case,'events':len(records),'output_bytes':len(outputs[1][1]),'identical':True})
    (args.output/'report.json').write_text(json.dumps({'seed':'0x3813e93b','cases':reports},indent=2)+'\n')
    print(f'{len(reports)} external C/Rust differential cases identical')


if __name__=='__main__':main()
