import { Construct } from "constructs";
import * as aws from "@cdktf/provider-aws/lib";

export interface NegyServiceProps {
  cluster: aws.ecsCluster.EcsCluster;
  vpc: aws.vpc.Vpc;
  subnets: aws.subnet.Subnet[];
  image: string;
  port: number;
  cpu: number;
  memory: number;
  command?: string[];
  tasks: number;
  isPublic: boolean,
  loadBalancer?: aws.ecsService.EcsServiceLoadBalancer[];
}

export class NegyService extends Construct {
  constructor(scope: Construct, name: string, props: NegyServiceProps) {
    super(scope, name);

    const taskExecutionRole = new aws.iamRole.IamRole(this, `TaskExecutionRole_${name}`, {
      name: `TaskExecutionRole_${name}`,
      assumeRolePolicy: JSON.stringify({
        Version: "2012-10-17",
        Statement: [
          {
            Action: "sts:AssumeRole",
            Effect: "Allow",
            Sid: "",
            Principal: {
              Service: "ecs-tasks.amazonaws.com",
            },
          },
        ],
      }),
    });

    new aws.iamRolePolicyAttachment.IamRolePolicyAttachment(this, `TaskExecutionRolePolicy_${name}`, {
      role: taskExecutionRole.id,
      policyArn: "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy",
    });

    const taskRole = new aws.iamRole.IamRole(this, `TaskRole_${name}`, {
      name: `TaskRole_${name}`,
      inlinePolicy: [{
        name: `Logs_${name}`,
        policy: JSON.stringify({
          Version: "2012-10-17",
          Statement: [
            {
              Effect: "Allow",
              Action: ["logs:CreateLogStream", "logs:PutLogEvents"],
              Resource: "*",
            },
          ],
        }),
      }],
      assumeRolePolicy: JSON.stringify({
        Version: "2012-10-17",
        Statement: [
          {
            Action: "sts:AssumeRole",
            Effect: "Allow",
            Sid: "",
            Principal: {
              Service: "ecs-tasks.amazonaws.com",
            },
          },
        ],
      }),
    });

    const logGroup = new aws.cloudwatchLogGroup.CloudwatchLogGroup(this, `LogGroup_${name}`, {
      name: `${props.cluster.name}/${name}`,
      retentionInDays: 30,
    });

    const taskDefinition = new aws.ecsTaskDefinition.EcsTaskDefinition(this, `TaskDefinition_${name}`, {
      family: name,
      cpu: `${props.cpu}`,
      memory: `${props.memory}`,
      requiresCompatibilities: ["FARGATE"],
      networkMode: "awsvpc",
      executionRoleArn: taskExecutionRole.arn,
      taskRoleArn: taskRole.arn,
      containerDefinitions: JSON.stringify([
        {
          name,
          image: props.image,
          cpu: props.cpu,
          memory: props.memory,
          command: props.command,
          portMappings: [{
            protocol: "tcp",
            containerPort: props.port,
            hostPort: props.port,
          }],
          logConfiguration: {
            logDriver: "awslogs",
            options: {
              "awslogs-group": logGroup.name,
              "awslogs-region": "us-east-1",
              "awslogs-stream-prefix": "node-pool",
            },
          },
        }
      ])
    });

    const securityGroup = new aws.securityGroup.SecurityGroup(this, `ServiceSecurityGroup_${name}`, {
      name: `SecurityGroup_${name}`,
      vpcId: props.vpc.id,
    });

    new aws.securityGroupRule.SecurityGroupRule(this, `Ingress_${name}`, {
      fromPort: props.port,
      toPort: props.port,
      protocol: "TCP",
      securityGroupId: securityGroup.id,
      type: "ingress",
      cidrBlocks: ["0.0.0.0/0"],
    });

    new aws.securityGroupRule.SecurityGroupRule(this, `Exgress_${name}`, {
      fromPort: 0,
      toPort: 0,
      protocol: "-1",
      securityGroupId: securityGroup.id,
      type: "egress",
      cidrBlocks: ["0.0.0.0/0"],
    });

    new aws.ecsService.EcsService(this, `Service_${name}`, {
      name,
      cluster: props.cluster.arn,
      taskDefinition: taskDefinition.arn,
      desiredCount: props.tasks,
      launchType: "FARGATE",
      networkConfiguration: {
        assignPublicIp: props.isPublic,
        subnets: props.subnets.map(s => s.id),
        securityGroups: [securityGroup.id],
      },
      loadBalancer: props.loadBalancer,
    });
  }
}
