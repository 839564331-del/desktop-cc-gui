// @vitest-environment jsdom
import { renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useTriggerDetection } from './useTriggerDetection';

describe('useTriggerDetection', () => {
  it('detects $ skill trigger at token boundary', () => {
    const { result } = renderHook(() => useTriggerDetection());
    const text = '请使用 $wf-th';
    const trigger = result.current.detectTrigger(text, text.length);
    expect(trigger).toEqual({
      trigger: '$',
      query: 'wf-th',
      start: 4,
      end: text.length,
    });
  });

  it('does not detect $ trigger inside plain token', () => {
    const { result } = renderHook(() => useTriggerDetection());
    const text = 'price$wf-th';
    const trigger = result.current.detectTrigger(text, text.length);
    expect(trigger).toBeNull();
  });
});
