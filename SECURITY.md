# Security Policy

This document defines the policy how to report vulnerabilities and receive updates when security patches are released.

If you have any suggestions or comments about the security policy, please email the [Findora Security List](mailto:security@findora.org) at security@findora.org

## Reporting a vulnerability

All security issues and questions should be reported by sending email to [Findora Security List](mailto:security@findora.org) at security@findora.org.
This will be acknowledged within 24 hours by the Findora Team and kick of review process.
You will receive a more detailed response to the email within 72 hours indicating perceived severity and the next steps in handling your report.

After initial reply to your report, the security team will keep you informed about the progress toward patching and public disclosure.

## Handling & disclosure process

1. Security report is received and assigned to an owner. This person will coordinate process of evaluating, fixing, releasing and disclosing the issue.
2. After initial report received, the evaluation process is performed. Issue validity, severity, version and affected components will be assessed. Additional review to identify similar issues also happens. 
3. Fixes are implemented for all supported releases. These fixes are not publicly communicated but held in private repo of or locally.
4. A suggested announcement date for this vulnerability is chosen. The notification is drafted and includes patches to all supported versions and effected components.
5. On the announcement date, the announcement will be published via our regular channels. The changes are fast tracked and merged into the public repository. At least 6 hours after the mailing list is notified, a copy of the advisory will be published across social channels. 

This process may take time, especially when coordinating with network participants and maintainers of other components in the ecosystem.
The goal will be to address issues in as short period as possible, but it's important that the process described above to ensure that disclosures are handled in consistent manner.  

*Note:* If Security Team identifies that an issue is mission critical and requires subset of network participants to update prior to newsletter announcement - this will be done in manual way by communicating via direct channels. 

## Reward

The discovery of the security vulnerabilities that include but are not limited to the following categories will be rewarded proportionally to their severity:
* Algorithmic, implementation, and economic issues that violate safety of the blockchain;
* Algorithmic, implementation, and economic issues that stall the blockchain or significantly throttle liveness;
* Algorithmic, implementation, and economic issues in the standard contracts developed by Findora Foundation and maintainers;
* Issues that expose the private data of the users, the developers, or the validators;

The following are the neccessary conditions for the reward:
* The vulnerability is disclosed to Findora Foundation before it is disclosed publicly and Findora Foundation is given sufficient time to fix it;
* The vulnerability is not disclosed to anyone else except the finder and Findora Foundation before it is fixed;
* The vulnerability is not exploited at any time

### Rewards platform

We are using https://gitcoin.co/ to reward tokens. Meaning that every security vulnerability that you submit to us will be processed like a general work-item by an external contributor. To receive the reward you would need to register on https://gitcoin.co/ and be able to receive the reward through it.