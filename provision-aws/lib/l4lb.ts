import { Construct } from "constructs";
import * as aws from "@cdktf/provider-aws/lib";

export interface L4LbProps {
  vpc: aws.vpc.Vpc;
  subnets: aws.subnet.Subnet[];
  containerPort: number;
  listenerPort: number;
}

export class L4Lb extends Construct {
  public readonly lb: aws.lb.Lb;
  public readonly targetGroup: aws.lbTargetGroup.LbTargetGroup;

  constructor(scope: Construct, name: string, props: L4LbProps) {
    super(scope, name);

    const lb = new aws.lb.Lb(this, `L4Lb_${name}`, {
      name,
      internal: false,
      loadBalancerType: "network",
      subnets: props.subnets.map(s => s.id),
    });

    const targetGroup = new aws.lbTargetGroup.LbTargetGroup(this, `L4LbTargetGroup_${name}`, {
      name: `TargetGroup${name}`,
      port: props.containerPort,
      protocol: "TCP",
      targetType: "ip",
      vpcId: props.vpc.id,
      healthCheck: {
        enabled: true,
        protocol: "TCP",
      },
    });

    new aws.lbListener.LbListener(this, `L4LbListener_${name}`, {
      loadBalancerArn: lb.arn,
      port: props.listenerPort,
      protocol: "TCP",
      defaultAction: [
        {
          type: "forward",
          targetGroupArn: targetGroup.arn,
        },
      ],
    });

    this.lb = lb;
    this.targetGroup = targetGroup;
  }
}
