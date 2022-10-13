import { Construct } from "constructs";
import * as aws from "@cdktf/provider-aws/lib";

export interface L7LbProps {
  vpc: aws.vpc.Vpc;
  subnets: aws.subnet.Subnet[];
  containerPort: number;
  listenerPort: number;
  healthCheck: aws.lbTargetGroup.LbTargetGroupHealthCheck,
}

export class L7Lb extends Construct {
  public readonly lb: aws.lb.Lb;
  public readonly targetGroup: aws.lbTargetGroup.LbTargetGroup;

  constructor(scope: Construct, name: string, props: L7LbProps) {
    super(scope, name);

    const securityGroup = new aws.securityGroup.SecurityGroup(this, `L7LbSecurityGroup_${name}`, {
      name: `L7LbSecurityGroup_${name}`,
      vpcId: props.vpc.id,
      ingress: [
        {
          fromPort: props.listenerPort,
          toPort: props.listenerPort,
          protocol: "TCP",
          cidrBlocks: ["0.0.0.0/0"]
        },
      ],
      egress: [
        {
          fromPort: 0,
          toPort: 0,
          protocol: "-1",
          cidrBlocks: ["0.0.0.0/0"],
        },
      ],
    });

    const lb = new aws.lb.Lb(this, `L7Lb_${name}`, {
      name,
      internal: false,
      loadBalancerType: "application",
      securityGroups: [securityGroup.id],
      subnets: props.subnets.map(s => s.id),
    });

    const targetGroup = new aws.lbTargetGroup.LbTargetGroup(this, `L7LbTargetGroup_${name}`, {
      name: `TargetGroup${name}`,
      port: props.containerPort,
      protocol: "HTTP",
      targetType: "ip",
      vpcId: props.vpc.id,
      healthCheck: props.healthCheck,
    });

    new aws.lbListener.LbListener(this, `L7LbListener_${name}`, {
      loadBalancerArn: lb.arn,
      port: props.listenerPort,
      protocol: "HTTP",
      defaultAction: [
        {
          type: "forward",
          targetGroupArn: targetGroup.arn,
        }
      ],
    });

    this.lb = lb;
    this.targetGroup = targetGroup;
  }
}
