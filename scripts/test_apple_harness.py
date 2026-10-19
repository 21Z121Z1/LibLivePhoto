#!/usr/bin/env python3
"""Fault injection for fail-closed harness gates; never native acceptance."""
import json, subprocess, tempfile, unittest
from pathlib import Path
from unittest.mock import patch
import test_apple_simulator as harness

OWNED='11111111-2222-4333-8444-555555555555'
OTHER='aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee'

class AcceptanceGates(unittest.TestCase):
    def test_missing_runtime_cannot_leave_an_old_success_receipt(self):
        with tempfile.TemporaryDirectory(prefix='llp-harness-unit-') as directory:
            root=Path(directory);source=root/'apple/Acceptance/BridgeSimulator.swift'
            source.parent.mkdir(parents=True);source.write_text('// isolated unit fixture\n')
            out=root/'dist/apple/simulator-acceptance';out.mkdir(parents=True)
            receipt=out/'runtime-receipt.json';receipt.write_text('{"result":"passed"}')
            def command(args,**kwargs):
                if args[:2]==['xcrun','--sdk']:return '/unit-sdk\n'
                if args[:2]==['xcrun','swiftc']:return ''
                return '{"runtimes":[]}'
            with patch.object(harness,'ROOT',root),patch.object(harness,'run',side_effect=command):
                with self.assertRaisesRegex(RuntimeError,'no available iOS Simulator runtime'):harness.main([])
            self.assertEqual(json.loads(receipt.read_text())['result'],'not_executed')

    def scenario(self, state='Booted', process_failure=False, delete_failure=False):
        with tempfile.TemporaryDirectory(prefix='llp-harness-unit-') as directory:
            root=Path(directory)
            source=root/'apple/Acceptance/BridgeSimulator.swift'
            source.parent.mkdir(parents=True);source.write_text('// isolated unit fixture\n')
            invoked=[];cleanup=[]
            def command(args, **kwargs):
                if args[:2]==['git','rev-parse']:return 'unit-fault-injection-sha\n'
                if args[:2]==['xcrun','--sdk']:return '/unit-sdk\n'
                if args[:2]==['xcrun','swiftc']:return ''
                if args[2:4]==['list','runtimes']:
                    return json.dumps({'runtimes':[{'identifier':'com.apple.CoreSimulator.SimRuntime.iOS-26-5','version':'26.5','isAvailable':True}]})
                if args[2:4]==['list','devicetypes']:
                    return json.dumps({'devicetypes':[{'identifier':'unit-phone','name':'iPhone 17'}]})
                if args[2:4]==['list','devices']:
                    # A different booted device must never authorize recovery.
                    return json.dumps({'devices':{'unit-runtime':[{'udid':OTHER,'state':'Booted'},{'udid':OWNED,'state':state}]}})
                if args[2]=='create':return OWNED+'\n'
                if args[2]=='boot':return ''
                self.fail('unexpected command: '+str(args))
            def logged(args,path,timeout):
                invoked.append(args[2]);path.write_text('unit fault-injection log\n')
                if args[2]=='bootstatus':raise subprocess.TimeoutExpired(args,timeout)
                self.assertEqual(args[2],'spawn')
                if process_failure:raise subprocess.CalledProcessError(1,args)
                path.write_text('IOS_SIMULATOR_RUNTIME: fault-injection fixture, not iOS\nIOS_SIMULATOR_SWIFT_RUST passed: unit fixture only\n')
            def clean_command(args,**kwargs):
                cleanup.append(args)
                return subprocess.CompletedProcess(args,1 if delete_failure and args[2]=='delete' else 0)
            error=None
            with patch.object(harness,'ROOT',root),patch.object(harness,'run',side_effect=command),patch.object(harness,'logged',side_effect=logged),patch.object(harness,'diagnostics'),patch.object(harness.subprocess,'run',side_effect=clean_command):
                try:harness.main([])
                except (RuntimeError,subprocess.CalledProcessError) as caught:error=caught
            receipt=json.loads((root/'dist/apple/simulator-acceptance/runtime-receipt.json').read_text())
            self.assertEqual(cleanup,[['xcrun','simctl','shutdown',OWNED],['xcrun','simctl','delete',OWNED]])
            return receipt,error,invoked

    def test_booted_recovery_still_executes_the_original_process_gate(self):
        receipt,error,invoked=self.scenario()
        self.assertIsNone(error);self.assertEqual(invoked,['bootstatus','spawn'])
        self.assertFalse(receipt['bootstatus_completed']);self.assertTrue(receipt['runtime_process_passed'])
        self.assertEqual(receipt['result'],'passed');self.assertFalse(receipt['springboard_ui_acceptance'])

    def test_other_booted_device_cannot_turn_owned_shutdown_device_into_success(self):
        receipt,error,invoked=self.scenario(state='Shutdown')
        self.assertIsInstance(error,RuntimeError);self.assertEqual(invoked,['bootstatus'])
        self.assertEqual(receipt['result'],'failed');self.assertFalse(receipt['runtime_process_passed'])

    def test_native_assertion_failure_cannot_be_masked_by_booted_state(self):
        receipt,error,invoked=self.scenario(process_failure=True)
        self.assertIsInstance(error,subprocess.CalledProcessError);self.assertEqual(invoked,['bootstatus','spawn'])
        self.assertEqual(receipt['result'],'failed');self.assertFalse(receipt['runtime_process_passed'])

    def test_cleanup_failure_is_reported_after_a_passing_process(self):
        receipt,error,_=self.scenario(delete_failure=True)
        self.assertIsInstance(error,RuntimeError);self.assertEqual(receipt['result'],'failed')
        self.assertTrue(receipt['runtime_process_passed']);self.assertFalse(receipt['owned_device_deleted'])

if __name__=='__main__':unittest.main()
